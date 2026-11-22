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
//! The initializers of a function's parameters, and the patterns among its parameters with their
//! defaults, are the function's own code, walked before its body: their references and direct
//! `eval` calls count for the function, and whether they hold a direct `eval` call is kept apart
//! ([`Scope::parameters_contain_direct_eval`]). A binding pattern declares each of its
//! BoundNames as its declaration does (`var` or lexical names, or parameters).
//!
//! An arrow function is a function scope with no `arguments` of its own: its references to
//! `arguments`, where it binds no such name itself, and its direct `eval` calls count for the
//! nearest enclosing function that is not an arrow, which then gets its arguments object
//! ([`Scope::arguments_object`]; `docs/design/engine.md`, decision 3, "`this` and `arguments` in
//! an arrow function", item 2).
//!
//! A method is a function scope like any other; a `catch` without a parameter binds nothing, and
//! a label is no binding: neither changes what a body declares.
//!
//! The walk does not recurse: its pending work is a stack on the heap, so the Rust stack it uses
//! is the same at any depth of the tree.

use std::collections::BTreeSet;

use crate::ledger::{HeapBytes, Meter, Stop, map_entry_bytes};

use crate::ast::{
    Ast, BindingTarget, Child, DeclKind, Expr, ForInTarget, ForInit, Function, FunctionId,
    FunctionKind, LexicalName, Name, Scope, Stmt, StmtId,
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
    for name in declarators.iter().flat_map(|d| d.target.names()) {
        names.push(LexicalName {
            name: name.clone(),
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
///
/// Every structure of the analysis is charged to `m` before it grows: the scopes it keeps in
/// the tree by capacity, and its scratch (the sets of names seen, an entry each as
/// [`crate::ledger::map_entry_bytes`] with the bytes of each name, the declarations, the stack
/// of pending work and the bodies open), which it credits when it drops it. It calls
/// [`Meter::visit`] for every node and ends at the first stop; the entry then answers the stop.
pub(crate) fn analyse(ast: &mut Ast, body: &[StmtId], m: &Meter<'_>) -> Scope {
    let (script, mut scopes) = walk(ast, body, m);
    for (id, scope) in scopes.drain(..) {
        match ast.functions.get_mut(id.0 as usize) {
            Some(f) => {
                let old = std::mem::replace(&mut f.scope, scope);
                m.release(old);
            }
            None => m.release(scope),
        }
    }
    m.release(scopes);
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
    /// Whether the initializers of the function's parameters, walked before its body, hold a
    /// direct `eval` call.
    parameters_contain_direct_eval: bool,
    /// Whether an arrow function in the body, at any depth of arrows, references `arguments`
    /// with no arrow between binding it, or holds a direct `eval` call: the `arguments` of this
    /// body is then that arrow's too ("`this` and `arguments` in an arrow function", item 2).
    inner_arguments: bool,
}

impl HeapBytes for Collect {
    fn heap_bytes(&self) -> usize {
        self.var_names
            .heap_bytes()
            .saturating_add(self.var_seen.heap_bytes())
            .saturating_add(self.declarations.heap_bytes())
            .saturating_add(self.lexical.heap_bytes())
            .saturating_add(self.references.heap_bytes())
            .saturating_add(self.reference_seen.heap_bytes())
    }
}

impl HeapBytes for Step {
    fn heap_bytes(&self) -> usize {
        0
    }
}

/// A name new to `seen`: its entry and a copy charged, then a second copy pushed onto `list`.
fn add_name(m: &Meter<'_>, seen: &mut BTreeSet<String>, list: &mut Vec<String>, name: &str) {
    if seen.contains(name) {
        return;
    }
    let Ok(key) = m.string(name) else {
        return;
    };
    if !matches!(m.insert(seen, key), Ok(true)) {
        return;
    }
    if let Ok(copy) = m.string(name) {
        // A refusal stops the meter; the walk ends at its next node.
        let _ = m.push(list, copy);
    }
}

impl Collect {
    fn var(&mut self, m: &Meter<'_>, name: &str) {
        add_name(m, &mut self.var_seen, &mut self.var_names, name);
    }

    fn reference(&mut self, m: &Meter<'_>, name: &str) {
        add_name(m, &mut self.reference_seen, &mut self.references, name);
    }

    /// Whether the body of the arrow function with parameters `params` uses the `arguments` of
    /// the code around it: its own code, or an arrow in it, references the name and the arrow
    /// binds no `arguments` of its own (a parameter, a `var`, a function or a lexical
    /// declaration), or its own code holds a direct `eval` call, whose eval code may reference
    /// it (decision 3, "Names", item 4).
    fn arguments_of_outer_function(&self, params: &[BindingTarget]) -> bool {
        let binds = params
            .iter()
            .flat_map(BindingTarget::names)
            .any(|p| p.name == "arguments")
            || self.var_seen.contains("arguments")
            || self
                .declarations
                .iter()
                .any(|(name, _)| name == "arguments")
            || self.lexical.iter().any(|l| l.name.name == "arguments");
        ((self.reference_seen.contains("arguments") || self.inner_arguments) && !binds)
            || self.contains_direct_eval
    }

    /// The scope of the body whose walk ends; `function` is the function, `None` for the
    /// script; an arrow function has no arguments object
    /// (`#sec-functiondeclarationinstantiation`, step 16). The scratch is credited.
    fn finish(self, function: Option<&Function>, m: &Meter<'_>) -> Scope {
        // `#sec-functiondeclarationinstantiation`, steps 15 to 18: no arguments object when a
        // parameter is named `arguments`, or, in a list with no initializer, a function
        // declaration or a lexical declaration of the body; a `var` of that name keeps it. A
        // direct `eval` may reference it in code no analysis sees (decision 3, "Names", item
        // 4), and an arrow in the body may use it.
        let arguments_object = function.is_some_and(|f| {
            f.kind != FunctionKind::Arrow
                && (self.reference_seen.contains("arguments")
                    || self.contains_direct_eval
                    || self.inner_arguments)
                && !f.parameter_names().any(|p| p.name == "arguments")
                && (f.has_parameter_expressions()
                    || (!self
                        .declarations
                        .iter()
                        .any(|(name, _)| name == "arguments")
                        && !self.lexical.iter().any(|l| l.name.name == "arguments")))
        });
        // The last declaration of each name, in source order; each entry of the names taken is
        // charged when it is inserted and all are credited after.
        let entry = map_entry_bytes::<&str, ()>();
        let mut taken = BTreeSet::new();
        let mut functions = Vec::new();
        for (name, id) in self.declarations.iter().rev() {
            if !taken.contains(name.as_str()) {
                if m.charge(entry).is_err() {
                    break;
                }
                taken.insert(name.as_str());
                if m.push(&mut functions, *id).is_err() {
                    break;
                }
            }
        }
        m.credit(taken.len().saturating_mul(entry));
        drop(taken);
        functions.reverse();
        m.release(self.var_seen);
        m.release(self.declarations);
        m.release(self.reference_seen);
        Scope {
            var_names: self.var_names,
            functions,
            lexical: self.lexical,
            references: self.references,
            arguments_object,
            contains_with: self.contains_with,
            contains_direct_eval: self.contains_direct_eval,
            parameters_contain_direct_eval: self.parameters_contain_direct_eval,
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
    /// The end of the initializers of a function's parameters, before its body.
    Parameters,
    /// The end of a function's body: its names become its scope.
    End(FunctionId),
}

/// Pushes the statements of a body, so that each one's top-level step comes before its walk;
/// the growth of the stack charged first.
fn push_body(m: &Meter<'_>, stack: &mut Vec<Step>, body: &[StmtId]) -> Result<(), Stop> {
    m.grow_by(stack, body.len().saturating_mul(2))?;
    for s in body.iter().rev() {
        stack.push(Step::Node(Child::Stmt(*s)));
        stack.push(Step::TopLevel(*s));
    }
    Ok(())
}

/// The names the statement `s` binds when it is a `let` or `const` declaration, each a copy
/// charged to `m`, onto `names`.
fn push_lexical_charged(m: &Meter<'_>, ast: &Ast, s: StmtId, names: &mut Vec<LexicalName>) {
    let Some(Stmt::Var(kind, declarators)) = ast.stmt(s).map(|n| &n.stmt) else {
        return;
    };
    if !kind.is_lexical() {
        return;
    }
    for bound in declarators.iter().flat_map(|d| d.target.names()) {
        let Ok(name) = m.string(&bound.name) else {
            return;
        };
        let lexical = LexicalName {
            name: Name {
                name,
                span: bound.span,
            },
            constant: *kind == DeclKind::Const,
            declaration: s,
        };
        if m.push(names, lexical).is_err() {
            return;
        }
    }
}

/// Walks the script `body` and the functions in it, each node before its children and the
/// children in source order; answers the script's scope and each function's.
fn walk(ast: &Ast, body: &[StmtId], m: &Meter<'_>) -> (Scope, Vec<(FunctionId, Scope)>) {
    let mut scopes = Vec::new();
    let mut script = Collect::default();
    // The bodies being walked inside the script, innermost last.
    let mut open: Vec<Collect> = Vec::new();
    let mut stack = Vec::new();
    if push_body(m, &mut stack, body).is_ok() {
        while let Some(step) = stack.pop() {
            if walk_step(
                ast,
                m,
                step,
                &mut script,
                &mut open,
                &mut stack,
                &mut scopes,
            )
            .is_err()
            {
                break;
            }
        }
    }
    m.release(stack);
    m.release(open);
    (script.finish(None, m), scopes)
}

/// One step of [`walk`].
fn walk_step(
    ast: &Ast,
    m: &Meter<'_>,
    step: Step,
    script: &mut Collect,
    open: &mut Vec<Collect>,
    stack: &mut Vec<Step>,
    scopes: &mut Vec<(FunctionId, Scope)>,
) -> Result<(), Stop> {
    let c = open.last_mut().unwrap_or(script);
    match step {
        Step::TopLevel(s) => {
            if let Some(Stmt::FunctionDeclaration(f)) = ast.stmt(s).map(|n| &n.stmt)
                && let Some(name) = ast.function(*f).and_then(|x| x.name.as_ref())
            {
                c.var(m, &name.name);
                let copy = m.string(&name.name)?;
                m.push(&mut c.declarations, (copy, *f))?;
            }
            push_lexical_charged(m, ast, s, &mut c.lexical);
        }
        Step::Parameters => c.parameters_contain_direct_eval = c.contains_direct_eval,
        Step::End(id) => {
            if let Some(done) = open.pop() {
                let f = ast.function(id);
                let params = f.map(|f| f.params.as_slice()).unwrap_or(&[]);
                let arrow = f.is_some_and(|f| f.kind == FunctionKind::Arrow);
                // An arrow has no `arguments` of its own: what its code uses of it counts for
                // the body around it, up to the nearest function that is not an arrow.
                if arrow && done.arguments_of_outer_function(params) {
                    open.last_mut().unwrap_or(script).inner_arguments = true;
                }
                let scope = done.finish(f, m);
                m.push(scopes, (id, scope))?;
            }
        }
        Step::Node(Child::Function(id)) => {
            m.visit()?;
            // A function is analysed on its own: the initializers of its parameters, then its
            // body.
            let Some(f) = ast.function(id) else {
                return Ok(());
            };
            m.push(open, Collect::default())?;
            m.push(stack, Step::End(id))?;
            push_body(m, stack, &f.body)?;
            // The patterns of the parameters, with their defaults, are the parameters' code
            // as the initializers are.
            let patterns = f.params.iter().filter_map(BindingTarget::pattern);
            m.grow_by(
                stack,
                f.initializers
                    .len()
                    .saturating_add(patterns.clone().count())
                    .saturating_add(1),
            )?;
            stack.push(Step::Parameters);
            for e in f.initializers.iter().rev().flatten() {
                stack.push(Step::Node(Child::Expr(*e)));
            }
            for e in patterns {
                stack.push(Step::Node(Child::Expr(e)));
            }
        }
        Step::Node(node @ Child::Stmt(id)) => {
            m.visit()?;
            collect_stmt(ast, id, c, m);
            ast.push_children_charged(node, stack, Step::Node, m)?;
        }
        Step::Node(node @ Child::Expr(id)) => {
            m.visit()?;
            // An identifier is a reference of the body being analysed; a direct `eval` call
            // is the body's own.
            match ast.expr(id).map(|n| &n.expr) {
                Some(Expr::Identifier(n)) => c.reference(m, n),
                Some(Expr::Call { callee, .. }) if ast.is_direct_eval_callee(*callee) => {
                    c.contains_direct_eval = true;
                }
                _ => {}
            }
            ast.push_children_charged(node, stack, Step::Node, m)?;
        }
    }
    m.check()
}

/// The `var` names the statement `id` declares itself (`VarDeclaredNames` of a variable
/// statement, a `for` with `var` and a for-in with `var`; a `let` or `const` declaration has
/// none).
fn collect_stmt(ast: &Ast, id: StmtId, c: &mut Collect, m: &Meter<'_>) {
    match ast.stmt(id).map(|n| &n.stmt) {
        Some(
            Stmt::Var(DeclKind::Var, declarators)
            | Stmt::For {
                init: Some(ForInit::Var(DeclKind::Var, declarators)),
                ..
            },
        ) => {
            for name in declarators.iter().flat_map(|d| d.target.names()) {
                c.var(m, &name.name);
            }
        }
        Some(Stmt::ForIn {
            target: ForInTarget::Var(DeclKind::Var, target),
            ..
        }) => {
            for name in target.names() {
                c.var(m, &name.name);
            }
        }
        Some(Stmt::With { .. }) => c.contains_with = true,
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use crate::ledger::{Counting, Meter, map_entry_bytes};
    use crate::{ParseOptions, parse_script};

    /// A source that declares 10,000 distinct names with `var` and refers to each: while the
    /// scope analysis runs, its two sets (`var_seen` and `reference_seen`) hold every name, each
    /// entry charged the per-entry constant and each name its bytes.
    #[test]
    fn the_scope_sets_are_charged_per_entry() {
        let mut text = String::new();
        let mut name_bytes = 0;
        for i in 0..10_000 {
            let name = format!("name{i}");
            name_bytes += name.len();
            text.push_str(&format!("var {name}; {name};\n"));
        }
        let source: Vec<u16> = text.encode_utf16().collect();
        let mut script = parse_script(&source, ParseOptions { max_nesting: 100 }).unwrap();
        let mut ledger = Counting::default();
        {
            let meter = Meter::new(&mut ledger);
            let scope = super::analyse(&mut script.ast, &script.body, &meter);
            assert_eq!(scope.var_names.len(), 10_000);
            assert_eq!(scope.references.len(), 10_000);
        }
        let floor = 20_000 * map_entry_bytes::<String, ()>() + 2 * name_bytes;
        println!(
            "high-water {} for a floor of {floor} (entry {})",
            ledger.high,
            map_entry_bytes::<String, ()>()
        );
        assert!(
            ledger.high >= floor,
            "high-water {} below {floor}",
            ledger.high
        );
    }
}
