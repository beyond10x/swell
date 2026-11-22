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
//!   identifiers its own code references; whether a function needs an arguments object
//!   (`#sec-functiondeclarationinstantiation`, steps 15 to 18), whether a body holds a `with`
//!   statement (`#sec-with-statement`), and whether it holds a direct `eval` call
//!   (`#sec-function-calls-runtime-semantics-evaluation`, step 6), which also gives a function its
//!   arguments object (`docs/design/engine.md`, decision 3, "Names", item 4).
//! - LexicallyDeclaredNames and LexicallyScopedDeclarations
//!   (`#sec-static-semantics-lexicallydeclarednames`,
//!   `#sec-static-semantics-lexicallyscopeddeclarations`) of the top level of the script and of
//!   each function body ([`Scope::lexical`]), and of any statement list, a block's or the clauses
//!   of a `case` block ([`Ast::lexical_names`]): the `let` and `const` declarations standing in
//!   the list itself. A function declaration in a block is refused before a later story of slice
//!   6, and one at the top level of a body is var scoped (TopLevelLexicallyScopedDeclarations).
//!
//! A method is a function scope like any other; a `catch` without a parameter binds nothing, and
//! a label is no binding: neither changes what a body declares.
//!
//! The walk does not recurse: its pending work is a stack on the heap, so the Rust stack it uses
//! is the same at any depth of the tree.

use std::collections::BTreeSet;

use crate::ast::{
    Ast, Child, DeclKind, Expr, ForInTarget, ForInit, FunctionId, LexicalName, Name, Scope, Stmt,
    StmtId,
};

impl Ast {
    /// The names the `let` and `const` declarations of the statement list `list` bind, in
    /// source order: the BoundNames of its LexicallyScopedDeclarations
    /// (`#sec-block-static-semantics-lexicallyscopeddeclarations`), for a block, the clauses of
    /// a `case` block taken together or the top level of a body.
    pub fn lexical_names(&self, list: &[StmtId]) -> Vec<LexicalName> {
        let mut names = Vec::new();
        for &s in list {
            push_lexical(self, s, &mut names);
        }
        names
    }
}

/// The names the statement `s` binds when it is a `let` or `const` declaration.
fn push_lexical(ast: &Ast, s: StmtId, names: &mut Vec<LexicalName>) {
    let Some(Stmt::Var(kind, declarators)) = ast.stmt(s).map(|n| &n.stmt) else {
        return;
    };
    if !kind.is_lexical() {
        return;
    }
    for d in declarators {
        names.push(LexicalName {
            name: d.name.clone(),
            constant: *kind == DeclKind::Const,
            declaration: s,
        });
    }
}

/// Whether `raw`, the source text of a string literal in a directive prologue, is a Use Strict
/// Directive.
pub(crate) fn is_use_strict(raw: &[u16]) -> bool {
    let double = "\"use strict\"".encode_utf16();
    let single = "'use strict'".encode_utf16();
    raw.iter().copied().eq(double) || raw.iter().copied().eq(single)
}

/// Analyses the script `body` and every function of `ast`; stores each function's scope in it
/// and returns the script's.
pub(crate) fn analyse(ast: &mut Ast, body: &[StmtId]) -> Scope {
    let (script, scopes) = walk(ast, body);
    for (id, scope) in scopes {
        if let Some(f) = ast.functions.get_mut(id.0 as usize) {
            f.scope = scope;
        }
    }
    script
}

/// The names one body collects.
#[derive(Default)]
struct Collect {
    var_names: Vec<String>,
    var_seen: BTreeSet<String>,
    declarations: Vec<(String, FunctionId)>,
    lexical: Vec<LexicalName>,
    references: Vec<String>,
    reference_seen: BTreeSet<String>,
    contains_with: bool,
    contains_direct_eval: bool,
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

    /// The scope of the body whose walk ends; `params` are the parameters of a function, `None`
    /// for the script.
    fn finish(self, params: Option<&[Name]>) -> Scope {
        // `#sec-functiondeclarationinstantiation`, steps 15 to 18: no arguments object when a
        // parameter, a function declaration or a lexical declaration of the body is named
        // `arguments`; a `var` of that name keeps it. A direct `eval` may reference it in code
        // no analysis sees (decision 3, "Names", item 4).
        let arguments_object = params.is_some_and(|params| {
            (self.reference_seen.contains("arguments") || self.contains_direct_eval)
                && !params.iter().any(|p| p.name == "arguments")
                && !self
                    .declarations
                    .iter()
                    .any(|(name, _)| name == "arguments")
                && !self.lexical.iter().any(|l| l.name.name == "arguments")
        });
        // The last declaration of each name, in source order.
        let mut taken = BTreeSet::new();
        let mut functions = Vec::new();
        for (name, id) in self.declarations.iter().rev() {
            if taken.insert(name.as_str()) {
                functions.push(*id);
            }
        }
        functions.reverse();
        Scope {
            var_names: self.var_names,
            functions,
            lexical: self.lexical,
            references: self.references,
            arguments_object,
            contains_with: self.contains_with,
            contains_direct_eval: self.contains_direct_eval,
        }
    }
}

/// One piece of the walk's pending work.
enum Step {
    /// A statement at the top level of a script or function body, before it is walked: a
    /// function declaration there is var scoped (`TopLevelVarDeclaredNames`), a `let` or
    /// `const` declaration lexically scoped to the body.
    TopLevel(StmtId),
    Node(Child),
    /// The end of a function's body: its names become its scope.
    End(FunctionId),
}

/// Pushes the statements of a body, so that each one's top-level step comes before its walk.
fn push_body(stack: &mut Vec<Step>, body: &[StmtId]) {
    for s in body.iter().rev() {
        stack.push(Step::Node(Child::Stmt(*s)));
        stack.push(Step::TopLevel(*s));
    }
}

/// Walks the script `body` and the functions in it, each node before its children and the
/// children in source order; answers the script's scope and each function's.
fn walk(ast: &Ast, body: &[StmtId]) -> (Scope, Vec<(FunctionId, Scope)>) {
    let mut scopes = Vec::new();
    let mut script = Collect::default();
    // The bodies being walked inside the script, innermost last.
    let mut open: Vec<Collect> = Vec::new();
    let mut stack = Vec::new();
    push_body(&mut stack, body);
    while let Some(step) = stack.pop() {
        let c = open.last_mut().unwrap_or(&mut script);
        match step {
            Step::TopLevel(s) => {
                if let Some(Stmt::FunctionDeclaration(f)) = ast.stmt(s).map(|n| &n.stmt)
                    && let Some(name) = ast.function(*f).and_then(|x| x.name.as_ref())
                {
                    c.var(&name.name);
                    c.declarations.push((name.name.clone(), *f));
                }
                push_lexical(ast, s, &mut c.lexical);
            }
            Step::End(id) => {
                if let Some(done) = open.pop() {
                    let params = ast.function(id).map(|f| f.params.as_slice()).unwrap_or(&[]);
                    scopes.push((id, done.finish(Some(params))));
                }
            }
            Step::Node(Child::Function(id)) => {
                // A function is analysed on its own.
                let Some(f) = ast.function(id) else {
                    continue;
                };
                open.push(Collect::default());
                stack.push(Step::End(id));
                push_body(&mut stack, &f.body);
            }
            Step::Node(node @ Child::Stmt(id)) => {
                collect_stmt(ast, id, c);
                ast.push_children(node, &mut stack, Step::Node);
            }
            Step::Node(node @ Child::Expr(id)) => {
                // An identifier is a reference of the body being analysed; a direct `eval` call
                // is the body's own.
                match ast.expr(id).map(|n| &n.expr) {
                    Some(Expr::Identifier(n)) => c.reference(n),
                    Some(Expr::Call { callee, .. }) if ast.is_direct_eval_callee(*callee) => {
                        c.contains_direct_eval = true;
                    }
                    _ => {}
                }
                ast.push_children(node, &mut stack, Step::Node);
            }
        }
    }
    (script.finish(None), scopes)
}

/// The `var` names the statement `id` declares itself (`VarDeclaredNames` of a variable
/// statement, a `for` with `var` and a for-in with `var`; a `let` or `const` declaration has
/// none).
fn collect_stmt(ast: &Ast, id: StmtId, c: &mut Collect) {
    match ast.stmt(id).map(|n| &n.stmt) {
        Some(
            Stmt::Var(DeclKind::Var, declarators)
            | Stmt::For {
                init: Some(ForInit::Var(DeclKind::Var, declarators)),
                ..
            },
        ) => {
            for d in declarators {
                c.var(&d.name.name);
            }
        }
        Some(Stmt::ForIn {
            target: ForInTarget::Var(DeclKind::Var, name),
            ..
        }) => c.var(&name.name),
        Some(Stmt::With { .. }) => c.contains_with = true,
        _ => {}
    }
}
