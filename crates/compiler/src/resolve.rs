//! Slot and environment assignment (`#sec-environment-records`), from the scope analysis of
//! `swell-syntax`.
//!
//! Slice 1 has no direct `eval`, no `with` and no `arguments` (the parser refuses them), so every
//! binding of a function is known when the script is compiled: the parameters and the
//! `VarDeclaredNames` of a function (`#sec-functiondeclarationinstantiation`), the parameter of
//! a `catch` clause (`#sec-runtime-semantics-catchclauseevaluation`) and the own name of a named
//! function expression (`#sec-runtime-semantics-instantiateordinaryfunctionexpression`). A name
//! is resolved as ResolveBinding does (`#sec-resolvebinding`), through the environments that
//! enclose it, innermost first:
//!
//! - a binding that only its own function's code uses is a local slot of that function's frame;
//! - a binding that a nested function uses lives in an environment: the function environment
//!   the call creates, the environment of the `catch` clause, or the one-binding environment of
//!   a named function expression, which is always an environment;
//! - a name no enclosing function declares is resolved at run time through the global
//!   environment, and so are the script's own `var` and function declarations.
//!
//! An environment is created only for a scope with a binding in it; the hop count of an access
//! counts those environments between the access and the binding.
//!
//! The walk over the tree keeps its work on a stack of its own, so it uses no Rust stack in
//! proportion to the depth of the tree.

use std::collections::BTreeMap;

use swell_num::literal::numeric_literal;
use swell_num::to_string::number_to_string;
use swell_syntax::Script;
use swell_syntax::ast::{
    Ast, Expr, ExprId, ForInTarget, ForInit, FunctionId, FunctionKind, PropertyKey, PropertyValue,
    Stmt, StmtId,
};

use crate::CompileError;
use crate::bytecode::Slot;

/// A place in the tree where a name is bound or referenced.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Site {
    /// An `IdentifierReference`.
    Expr(ExprId),
    /// The `n`th declarator of a `var` statement or of a `for (var ...;...;...)` head.
    VarDecl(StmtId, u32),
    /// The `var` binding of a for-in head.
    ForInVar(StmtId),
    /// The `n`th parameter of a function, from inside it.
    Param(FunctionId, u32),
    /// The binding a function declaration initializes, from the top of the enclosing function.
    Declaration(FunctionId),
    /// The parameter of the `catch` clause of a try statement, from inside the clause.
    CatchParam(StmtId),
}

/// Where a name resolves to, from one site.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Binding {
    Local(u32),
    Env {
        hops: u32,
        slot: u32,
    },
    /// The own name of a named function expression: an immutable binding.
    Immutable {
        hops: u32,
        slot: u32,
        name: String,
    },
    /// Resolved at run time in the global environment.
    Global(String),
}

/// What the compiler needs of a function beyond its code.
#[derive(Clone, Debug, Default)]
pub(crate) struct FunctionInfo {
    pub locals: u32,
    /// The names of the function environment's slots, in slot order.
    pub captured: Vec<String>,
    pub params: Vec<Slot>,
    pub self_binding: bool,
    /// The name NamedEvaluation or an accessor gives an anonymous function.
    pub given_name: Option<Vec<u16>>,
}

pub(crate) struct Resolution {
    pub sites: BTreeMap<Site, Binding>,
    pub script_locals: u32,
    /// Indexed by `FunctionId`.
    pub functions: Vec<FunctionInfo>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Function,
    Catch,
    SelfName,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Place {
    Unset,
    Local(u32),
    Env(u32),
}

struct ScopeData {
    parent: Option<usize>,
    kind: Kind,
    /// The function whose frame holds the scope's slots; `None` for the script.
    owner: Option<FunctionId>,
    names: BTreeMap<String, usize>,
    /// The scope's bindings in declaration order.
    order: Vec<usize>,
    env_slots: u32,
}

struct BindingData {
    scope: usize,
    name: String,
    captured: bool,
    place: Place,
}

enum Walk {
    Stmt(StmtId),
    Expr(ExprId),
    Function(FunctionId),
    Catch(StmtId),
    Leave,
}

struct Resolver<'a> {
    ast: &'a Ast,
    scopes: Vec<ScopeData>,
    bindings: Vec<BindingData>,
    current: usize,
    /// Each site with the scope it is resolved from and its name.
    occurrences: Vec<(Site, usize, String)>,
    given_names: BTreeMap<FunctionId, Vec<u16>>,
    /// The scope of the own name of each named function expression.
    self_scopes: BTreeMap<FunctionId, usize>,
    work: Vec<Walk>,
}

fn malformed(what: &str) -> CompileError {
    CompileError::Tree(what.to_string())
}

/// Resolves every name of `script`.
pub(crate) fn resolve(script: &Script) -> Result<Resolution, CompileError> {
    let mut r = Resolver {
        ast: &script.ast,
        scopes: vec![ScopeData {
            parent: None,
            kind: Kind::Function,
            owner: None,
            names: BTreeMap::new(),
            order: Vec::new(),
            env_slots: 0,
        }],
        bindings: Vec::new(),
        current: 0,
        occurrences: Vec::new(),
        given_names: BTreeMap::new(),
        self_scopes: BTreeMap::new(),
        work: Vec::new(),
    };
    r.push_list(&script.body);
    while let Some(task) = r.work.pop() {
        r.step(task)?;
    }
    r.mark_captured()?;
    let mut frames = r.allocate()?;
    let sites = r.locate()?;
    let script_locals = frames.remove(&None).unwrap_or(0);
    let mut functions = Vec::with_capacity(script.ast.functions().len());
    for (i, _) in script.ast.functions().iter().enumerate() {
        let id = FunctionId(u32::try_from(i).map_err(|_| CompileError::TooMany("functions"))?);
        let mut info = r.function_info(id, &sites)?;
        info.locals = frames.get(&Some(id)).copied().unwrap_or(0);
        info.given_name = r.given_names.remove(&id);
        functions.push(info);
    }
    Ok(Resolution {
        sites,
        script_locals,
        functions,
    })
}

impl<'a> Resolver<'a> {
    fn push_list(&mut self, list: &[StmtId]) {
        for s in list.iter().rev() {
            self.work.push(Walk::Stmt(*s));
        }
    }

    fn new_scope(&mut self, kind: Kind, owner: Option<FunctionId>) -> usize {
        self.scopes.push(ScopeData {
            parent: Some(self.current),
            kind,
            owner,
            names: BTreeMap::new(),
            order: Vec::new(),
            env_slots: 0,
        });
        self.current = self.scopes.len() - 1;
        self.current
    }

    /// Declares `name` in `scope` once.
    fn declare(&mut self, scope: usize, name: &str) {
        let index = self.bindings.len();
        let Some(s) = self.scopes.get_mut(scope) else {
            return;
        };
        if s.names.contains_key(name) {
            return;
        }
        s.names.insert(name.to_string(), index);
        s.order.push(index);
        self.bindings.push(BindingData {
            scope,
            name: name.to_string(),
            captured: false,
            place: Place::Unset,
        });
    }

    fn occur(&mut self, site: Site, name: &str) {
        self.occurrences
            .push((site, self.current, name.to_string()));
    }

    fn step(&mut self, task: Walk) -> Result<(), CompileError> {
        match task {
            Walk::Stmt(id) => self.stmt(id),
            Walk::Expr(id) => self.expr(id),
            Walk::Function(id) => self.function(id),
            Walk::Catch(id) => self.catch(id),
            Walk::Leave => {
                let parent = self
                    .scopes
                    .get(self.current)
                    .and_then(|s| s.parent)
                    .ok_or_else(|| malformed("a scope left twice"))?;
                self.current = parent;
                Ok(())
            }
        }
    }

    /// The anonymous function definition `e` is, if it is one (`#sec-isanonymousfunctiondefinition`).
    fn anonymous_function(&self, e: ExprId) -> Option<FunctionId> {
        match self.ast.expr(e).map(|n| &n.expr) {
            Some(Expr::Function(f)) => self
                .ast
                .function(*f)
                .filter(|x| x.name.is_none() && x.kind == FunctionKind::Expression)
                .map(|_| *f),
            _ => None,
        }
    }

    fn stmt(&mut self, id: StmtId) -> Result<(), CompileError> {
        let node = self
            .ast
            .stmt(id)
            .ok_or_else(|| malformed("a statement index"))?;
        match &node.stmt {
            Stmt::Block(list) => self.push_list(list),
            Stmt::Var(declarators)
            | Stmt::For {
                init: Some(ForInit::Var(declarators)),
                ..
            } => {
                let mut children = Vec::new();
                for (i, d) in declarators.iter().enumerate() {
                    let n = u32::try_from(i).map_err(|_| CompileError::TooMany("declarators"))?;
                    self.occur(Site::VarDecl(id, n), &d.name.name);
                    if let Some(init) = d.init {
                        if let Some(f) = self.anonymous_function(init) {
                            self.given_names
                                .insert(f, d.name.name.encode_utf16().collect());
                        }
                        children.push(Walk::Expr(init));
                    }
                }
                if let Stmt::For {
                    test, update, body, ..
                } = &node.stmt
                {
                    children.extend(test.map(Walk::Expr));
                    children.extend(update.map(Walk::Expr));
                    children.push(Walk::Stmt(*body));
                }
                self.work.extend(children.into_iter().rev());
            }
            Stmt::Empty | Stmt::Continue | Stmt::Break | Stmt::Debugger => {}
            Stmt::Expression(e) | Stmt::Throw(e) => self.work.push(Walk::Expr(*e)),
            Stmt::Return(e) => self.work.extend(e.map(Walk::Expr)),
            Stmt::If {
                test,
                consequent,
                alternate,
            } => {
                self.work.extend(alternate.map(Walk::Stmt));
                self.work.push(Walk::Stmt(*consequent));
                self.work.push(Walk::Expr(*test));
            }
            Stmt::DoWhile { body, test } => {
                self.work.push(Walk::Expr(*test));
                self.work.push(Walk::Stmt(*body));
            }
            Stmt::While { test, body } => {
                self.work.push(Walk::Stmt(*body));
                self.work.push(Walk::Expr(*test));
            }
            Stmt::For {
                init,
                test,
                update,
                body,
            } => {
                self.work.push(Walk::Stmt(*body));
                self.work.extend(update.map(Walk::Expr));
                self.work.extend(test.map(Walk::Expr));
                if let Some(ForInit::Expression(e)) = init {
                    self.work.push(Walk::Expr(*e));
                }
            }
            Stmt::ForIn {
                target,
                object,
                body,
            } => {
                self.work.push(Walk::Stmt(*body));
                self.work.push(Walk::Expr(*object));
                match target {
                    ForInTarget::Var(name) => self.occur(Site::ForInVar(id), &name.name),
                    ForInTarget::Expression(e) => self.work.push(Walk::Expr(*e)),
                }
            }
            Stmt::Switch {
                discriminant,
                cases,
            } => {
                for case in cases.iter().rev() {
                    self.push_list(&case.body);
                    self.work.extend(case.test.map(Walk::Expr));
                }
                self.work.push(Walk::Expr(*discriminant));
            }
            Stmt::Try {
                block,
                handler,
                finalizer,
            } => {
                if let Some(list) = finalizer {
                    self.push_list(list);
                }
                if let Some(h) = handler {
                    self.work.push(Walk::Leave);
                    self.push_list(&h.body);
                    self.work.push(Walk::Catch(id));
                }
                self.push_list(block);
            }
            Stmt::FunctionDeclaration(f) => self.work.push(Walk::Function(*f)),
        }
        Ok(())
    }

    /// Enters the scope of the `catch` clause of the try statement `id`.
    fn catch(&mut self, id: StmtId) -> Result<(), CompileError> {
        let Some(Stmt::Try {
            handler: Some(h), ..
        }) = self.ast.stmt(id).map(|n| &n.stmt)
        else {
            return Err(malformed("a catch clause"));
        };
        let owner = self.scopes.get(self.current).and_then(|s| s.owner);
        let scope = self.new_scope(Kind::Catch, owner);
        self.declare(scope, &h.param.name);
        self.occur(Site::CatchParam(id), &h.param.name);
        Ok(())
    }

    fn expr(&mut self, id: ExprId) -> Result<(), CompileError> {
        let node = self
            .ast
            .expr(id)
            .ok_or_else(|| malformed("an expression index"))?;
        let mut children: Vec<Walk> = Vec::new();
        match &node.expr {
            Expr::This
            | Expr::Null
            | Expr::Boolean(_)
            | Expr::Number(_)
            | Expr::String(_)
            | Expr::RegExp(_) => {}
            Expr::Identifier(name) => self.occur(Site::Expr(id), name),
            Expr::Array(elements) => {
                children.extend(elements.iter().flatten().map(|e| Walk::Expr(*e)))
            }
            Expr::Object(properties) => {
                for p in properties {
                    let key = property_key(&p.key)?;
                    match p.value {
                        PropertyValue::Init(e) => {
                            if let Some(f) = self.anonymous_function(e)
                                && !is_proto(&p.key)
                            {
                                self.given_names.insert(f, key);
                            }
                            children.push(Walk::Expr(e));
                        }
                        PropertyValue::Get(f) | PropertyValue::Set(f) => {
                            let prefix = if matches!(p.value, PropertyValue::Get(_)) {
                                "get "
                            } else {
                                "set "
                            };
                            let mut name: Vec<u16> = prefix.encode_utf16().collect();
                            name.extend(key);
                            self.given_names.insert(f, name);
                            children.push(Walk::Function(f));
                        }
                    }
                }
            }
            Expr::Function(f) => children.push(Walk::Function(*f)),
            Expr::Unary { operand, .. } | Expr::Update { operand, .. } => {
                children.push(Walk::Expr(*operand));
            }
            Expr::Binary { left, right, .. } | Expr::Logical { left, right, .. } => {
                children.push(Walk::Expr(*left));
                children.push(Walk::Expr(*right));
            }
            Expr::Conditional {
                test,
                consequent,
                alternate,
            } => {
                children.push(Walk::Expr(*test));
                children.push(Walk::Expr(*consequent));
                children.push(Walk::Expr(*alternate));
            }
            Expr::Assign { op, target, value } => {
                // `#sec-assignment-operators-runtime-semantics-evaluation`: NamedEvaluation when
                // the target IsIdentifierRef (not in parentheses) and the operator is `=`.
                if *op == swell_syntax::ast::AssignOp::Assign
                    && let Some(t) = self.ast.expr(*target)
                    && !t.parenthesized
                    && let Expr::Identifier(name) = &t.expr
                    && let Some(f) = self.anonymous_function(*value)
                {
                    self.given_names.insert(f, name.encode_utf16().collect());
                }
                children.push(Walk::Expr(*target));
                children.push(Walk::Expr(*value));
            }
            Expr::Sequence(list) => children.extend(list.iter().map(|e| Walk::Expr(*e))),
            Expr::Member { object, .. } => children.push(Walk::Expr(*object)),
            Expr::Index { object, index } => {
                children.push(Walk::Expr(*object));
                children.push(Walk::Expr(*index));
            }
            Expr::Call { callee, arguments } => {
                children.push(Walk::Expr(*callee));
                children.extend(arguments.iter().map(|e| Walk::Expr(*e)));
            }
            Expr::New { callee, arguments } => {
                children.push(Walk::Expr(*callee));
                children.extend(arguments.iter().flatten().map(|e| Walk::Expr(*e)));
            }
        }
        self.work.extend(children.into_iter().rev());
        Ok(())
    }

    /// Enters a function: the scope of its own name (a named function expression), then the
    /// scope of its parameters and `var` declarations.
    fn function(&mut self, id: FunctionId) -> Result<(), CompileError> {
        let f = self
            .ast
            .function(id)
            .ok_or_else(|| malformed("a function index"))?;
        if f.kind == FunctionKind::Expression
            && let Some(name) = &f.name
        {
            let owner = self.scopes.get(self.current).and_then(|s| s.owner);
            let scope = self.new_scope(Kind::SelfName, owner);
            self.declare(scope, &name.name);
            self.self_scopes.insert(id, scope);
            self.work.push(Walk::Leave);
        }
        let scope = self.new_scope(Kind::Function, Some(id));
        for (i, p) in f.params.iter().enumerate() {
            self.declare(scope, &p.name);
            let n = u32::try_from(i).map_err(|_| CompileError::TooMany("parameters"))?;
            self.occur(Site::Param(id, n), &p.name);
        }
        for name in &f.scope.var_names {
            self.declare(scope, name);
        }
        for d in &f.scope.functions {
            let name = self
                .ast
                .function(*d)
                .and_then(|x| x.name.as_ref())
                .ok_or_else(|| malformed("a function declaration without a name"))?;
            self.occur(Site::Declaration(*d), &name.name);
        }
        self.work.push(Walk::Leave);
        self.push_list(&f.body);
        Ok(())
    }

    /// The binding `name` resolves to from `scope`, if any.
    fn lookup(&self, mut scope: usize, name: &str) -> Option<usize> {
        loop {
            let s = self.scopes.get(scope)?;
            if let Some(b) = s.names.get(name) {
                return Some(*b);
            }
            scope = s.parent?;
        }
    }

    /// Marks a binding captured when a site of another function resolves to it; the own name of a
    /// function expression is always captured when it is used.
    fn mark_captured(&mut self) -> Result<(), CompileError> {
        let mut marks = Vec::new();
        for (_, scope, name) in &self.occurrences {
            let Some(b) = self.lookup(*scope, name) else {
                continue;
            };
            let binding = self.bindings.get(b).ok_or_else(|| malformed("a binding"))?;
            let home = self
                .scopes
                .get(binding.scope)
                .ok_or_else(|| malformed("a scope"))?;
            let from = self
                .scopes
                .get(*scope)
                .ok_or_else(|| malformed("a scope"))?;
            if home.kind == Kind::SelfName || home.owner != from.owner {
                marks.push(b);
            }
        }
        for b in marks {
            if let Some(binding) = self.bindings.get_mut(b) {
                binding.captured = true;
            }
        }
        Ok(())
    }

    /// Gives each binding its place; the number of local slots of each frame.
    fn allocate(&mut self) -> Result<BTreeMap<Option<FunctionId>, u32>, CompileError> {
        let mut frames: BTreeMap<Option<FunctionId>, u32> = BTreeMap::new();
        for scope in 0..self.scopes.len() {
            let (order, owner, kind) = match self.scopes.get(scope) {
                Some(s) => (s.order.clone(), s.owner, s.kind),
                None => continue,
            };
            let mut env_slots = 0u32;
            for b in order {
                let Some(binding) = self.bindings.get_mut(b) else {
                    continue;
                };
                if binding.captured {
                    binding.place = Place::Env(env_slots);
                    env_slots = env_slots
                        .checked_add(1)
                        .ok_or(CompileError::TooMany("environment slots"))?;
                } else if kind != Kind::SelfName {
                    let locals = frames.entry(owner).or_insert(0);
                    binding.place = Place::Local(*locals);
                    *locals = locals
                        .checked_add(1)
                        .ok_or(CompileError::TooMany("local slots"))?;
                }
            }
            if let Some(s) = self.scopes.get_mut(scope) {
                s.env_slots = env_slots;
            }
        }
        Ok(frames)
    }

    /// The binding of every site.
    fn locate(&self) -> Result<BTreeMap<Site, Binding>, CompileError> {
        let mut sites = BTreeMap::new();
        for (site, scope, name) in &self.occurrences {
            let binding = match self.lookup(*scope, name) {
                None => Binding::Global(name.clone()),
                Some(b) => {
                    let data = self.bindings.get(b).ok_or_else(|| malformed("a binding"))?;
                    let mut hops = 0u32;
                    let mut s = *scope;
                    while s != data.scope {
                        let here = self.scopes.get(s).ok_or_else(|| malformed("a scope"))?;
                        if here.env_slots > 0 {
                            hops = hops
                                .checked_add(1)
                                .ok_or(CompileError::TooMany("environment hops"))?;
                        }
                        s = here.parent.ok_or_else(|| malformed("a scope chain"))?;
                    }
                    let home = self
                        .scopes
                        .get(data.scope)
                        .ok_or_else(|| malformed("a scope"))?;
                    match (data.place, home.kind) {
                        (Place::Local(n), _) => Binding::Local(n),
                        (Place::Env(slot), Kind::SelfName) => Binding::Immutable {
                            hops,
                            slot,
                            name: data.name.clone(),
                        },
                        (Place::Env(slot), _) => Binding::Env { hops, slot },
                        (Place::Unset, _) => return Err(malformed("a binding with no place")),
                    }
                }
            };
            sites.insert(*site, binding);
        }
        Ok(sites)
    }

    fn function_info(
        &self,
        id: FunctionId,
        sites: &BTreeMap<Site, Binding>,
    ) -> Result<FunctionInfo, CompileError> {
        let mut info = FunctionInfo::default();
        let Some(f) = self.ast.function(id) else {
            return Ok(info);
        };
        for i in 0..f.params.len() {
            let n = u32::try_from(i).map_err(|_| CompileError::TooMany("parameters"))?;
            let slot = match sites.get(&Site::Param(id, n)) {
                Some(Binding::Local(s)) => Slot::Local(*s),
                Some(Binding::Env { slot, .. }) => Slot::Env(*slot),
                _ => return Err(malformed("a parameter with no slot")),
            };
            info.params.push(slot);
        }
        for s in &self.scopes {
            if s.owner == Some(id) && s.kind == Kind::Function {
                let mut captured: Vec<(u32, String)> = s
                    .order
                    .iter()
                    .filter_map(|b| self.bindings.get(*b))
                    .filter_map(|b| match b.place {
                        Place::Env(slot) => Some((slot, b.name.clone())),
                        _ => None,
                    })
                    .collect();
                captured.sort();
                info.captured = captured.into_iter().map(|(_, n)| n).collect();
            }
        }
        info.self_binding = self
            .self_scopes
            .get(&id)
            .and_then(|s| self.scopes.get(*s))
            .is_some_and(|s| s.env_slots > 0);
        Ok(info)
    }
}

/// Whether a property name is `__proto__` (`#sec-object-initializer-runtime-semantics-propertydefinitionevaluation`):
/// an identifier or a string literal; a numeric key never is.
fn is_proto(key: &PropertyKey) -> bool {
    match key {
        PropertyKey::Identifier(s) => s == "__proto__",
        PropertyKey::String(s) => s.value.iter().copied().eq("__proto__".encode_utf16()),
        PropertyKey::Number(_) => false,
    }
}

/// The property key of a property name, as UTF-16 code units: an identifier's name, a string's
/// value, or a number's ToString (`#sec-object-initializer-runtime-semantics-evaluation`,
/// `LiteralPropertyName`).
pub(crate) fn property_key(key: &PropertyKey) -> Result<Vec<u16>, CompileError> {
    Ok(match key {
        PropertyKey::Identifier(s) => s.encode_utf16().collect(),
        PropertyKey::String(s) => s.value.clone(),
        PropertyKey::Number(n) => {
            let value = numeric_literal(n.text.as_bytes())
                .ok_or_else(|| malformed("a numeric literal swell-num does not read"))?
                .value;
            number_to_string(value).encode_utf16().collect()
        }
    })
}
