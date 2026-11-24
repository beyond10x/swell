//! Slot and environment assignment (`#sec-environment-records`), from the scope analysis of
//! `swell-syntax`.
//!
//! The bindings the compiler knows are the parameters (each name a parameter pattern binds, a
//! site of the pattern, [`Site::Bound`]), the binding `arguments` of a function
//! that needs an arguments object and the `VarDeclaredNames` of a function
//! (`#sec-functiondeclarationinstantiation`), the
//! parameter of a `catch` clause (`#sec-runtime-semantics-catchclauseevaluation`) and the own
//! name of a named function expression
//! (`#sec-runtime-semantics-instantiateordinaryfunctionexpression`). A name is resolved as
//! ResolveBinding does (`#sec-resolvebinding`), through the environments that enclose it,
//! innermost first:
//!
//! - a binding that only its own function's code uses is a local slot of that function's frame;
//! - a binding that a nested function uses lives in an environment: the function environment
//!   the call creates, the environment of the `catch` clause, or the one-binding environment of
//!   a named function expression, which is always an environment;
//! - a name no enclosing function declares is resolved at run time through the global
//!   environment, and so are the script's own `var` and function declarations.
//!
//! An environment is created only for a scope with a binding in it, and for each `with`
//! statement (its Object Environment Record); the hop count of an access counts those
//! environments between the access and the binding.
//!
//! A `with` statement makes what a name means depend on the run (`docs/design/engine.md`,
//! decision 3, "Names"):
//!
//! - a reference with a `with` body between it and the scope that binds it (the global scope when
//!   none does) is looked up by name at run time ([`Binding::Dynamic`]); a function nested in the
//!   body is inside it, and a reference whose binding lies inside the same body keeps its slot;
//! - every binding of every scope that encloses a `with` body, up to the global scope, lives in
//!   an environment whose slots have names: the function environment (named by
//!   `FunctionTemplate::captured`), the environment of a `catch` clause (a named one,
//!   `PushNamedEnv`) and that of a named function expression's own name; references to it from
//!   outside the body keep its slot.
//!
//! A direct `eval` call in non-strict code may add `var` and function bindings to the variable
//! environment of its caller at run time (`#sec-performeval`, `#sec-evaldeclarationinstantiation`),
//! which the same rule covers:
//!
//! - a reference with a function or script scope between it and its binding scope whose own code
//!   is non-strict and contains a direct `eval` call is looked up by name; a call in a nested
//!   function does not count for the outer scope, and a strict function keeps its slots even when
//!   it calls `eval` itself;
//! - every binding of every scope that encloses a direct `eval` call, strict or not, the scope of
//!   the call included, lives in an environment whose slots have names, so that the eval code
//!   finds it by name;
//! - in eval code ([`crate::compile_eval`]), a reference that the eval code does not bind is looked
//!   up by name, through the environments of the code that called `eval`.
//!
//! The `let` and `const` declarations make scopes of their own (`docs/design/engine.md`, decision
//! 3, "Lexical bindings"): a block, the case block of a `switch`, the head of a `for`, the
//! expression of a for-in head and each of its iterations, a `try`, `catch` or `finally` block,
//! the top level of a function body (inside the function's scope) and the top level of eval
//! code; each only when it declares a name. A script's own lexical names are names of the global
//! environment. A binding of such a scope is a local slot or a slot of a record the code makes
//! at the entry of the scope ([`LexicalScope`]), as the bindings of a `catch` clause are; an
//! access checks the binding's state ([`Binding::Lexical`]) unless it is in the code of the same
//! function after the end of the declaration, in a block, a function body or eval code.
//!
//! A function declaration in a block, a `case` block or a `try`, `catch` or `finally` block is
//! a binding of that scope (`docs/design/engine.md`, decision 3, "Function declarations in a
//! block"), never a `var` of the function, the script or the eval code: a local slot, or a slot
//! of the scope's record when a nested function (the function itself, calling itself by name)
//! refers to it or a lookup by name can reach it. The code makes its function object at the
//! entry of the scope ([`Site::Declaration`]), before any access, so no access checks it.
//!
//! A function whose parameter list is not simple (`docs/design/engine.md`, decision 3, "A
//! parameter list that is not simple"; `#sec-functiondeclarationinstantiation`) binds its
//! parameters in order in the scope of its parameters, where the initializers are resolved: an
//! access before the end of a parameter, its initializer included, or from another function is
//! checked, as a `let` binding's is. When a parameter has an initializer, the body's `var` names
//! are a scope of their own inside it (step 28), a `var` of a parameter's name or of `arguments`
//! starting with its value ([`Site::BodyVar`]), and the arguments object of such a list is
//! unmapped (step 22).
//!
//! A record whose slots have no names is passed by a lookup: no lookup can find its name there,
//! since a name a record between the reference and the `with` binds would have resolved the
//! reference at compile time. The parameters of a function that makes a mapped arguments object
//! live in its function environment, whose slots the object reads and writes
//! (`#sec-createmappedargumentsobject`).
//!
//! The walk over the tree keeps its work on a stack of its own, so it uses no Rust stack in
//! proportion to the depth of the tree.

use std::collections::{BTreeMap, BTreeSet};

use swell_num::literal::numeric_literal;
use swell_num::to_string::number_to_string;
use swell_syntax::Script;
use swell_syntax::ast::{
    Ast, BindingTarget, DeclKind, Expr, ExprId, ForInTarget, ForInit, FunctionId, FunctionKind,
    LexicalName, PatternTarget, PropertyKey, PropertyValue, Stmt, StmtId,
};
use swell_syntax::ledger::{HeapBytes, Meter, Stop, map_entry_bytes};

use crate::CompileError;
use crate::bytecode::{ArgumentsObject, Slot};

/// A scope of `let` and `const` bindings, by the node that makes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ScopeKey {
    /// The statement list of a block statement.
    Block(StmtId),
    /// The case block of a `switch`, entered after the discriminant.
    Case(StmtId),
    /// The head of a `for (let ...; ...; ...)` or `for (const ...; ...; ...)`.
    Head(StmtId),
    /// The expression of a for-in head with `let` or `const`: the head's names, uninitialized
    /// (`#sec-runtime-semantics-forinofheadevaluation`, step 2).
    Tdz(StmtId),
    /// One iteration of a for-in with `let` or `const`.
    Iteration(StmtId),
    /// The try block of a try statement.
    Try(StmtId),
    /// The catch block, inside the scope of the `catch` parameter.
    Catch(StmtId),
    /// The names a `catch` parameter that is a pattern binds, uninitialized until the pattern
    /// initializes them (`#sec-runtime-semantics-catchclauseevaluation`, steps 2 to 4).
    CatchParameter(StmtId),
    /// The finally block.
    Finally(StmtId),
    /// The top level of a function body, inside the function's scope.
    Body(FunctionId),
}

/// What the code does at the entry of a lexical scope.
#[derive(Clone, Debug, Default)]
pub(crate) struct LexicalScope {
    /// The names of the slots of the record it makes, in slot order, each with whether it is
    /// `const`; empty when it makes none.
    pub env: Vec<(String, bool)>,
    /// The local slots it sets uninitialized: those an access checks.
    pub uninitialize: Vec<u32>,
}

/// Where a `let` or `const` binding lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LexicalPlace {
    Local(u32),
    Env { hops: u32, slot: u32 },
}

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
    /// The `BindingIdentifier` of property `n` of the binding pattern node: the binding it
    /// initializes, or for a `var` pattern the binding it resolves before the property is read.
    Bound(ExprId, u32),
    /// The binding a function declaration initializes, from the top of the enclosing function,
    /// or for one of a block or a `case` block from the entry of that scope.
    Declaration(FunctionId),
    /// The parameter of the `catch` clause of a try statement, from inside the clause.
    CatchParam(StmtId),
    /// The binding `arguments` of a function that needs an arguments object, from the top of
    /// the function.
    Arguments(FunctionId),
    /// The binding of the `n`th of the `var` names of a function whose parameters have an
    /// initializer, in the `var` scope of its body, when a parameter or `arguments` has the same
    /// name: it starts with that binding's value (`#sec-functiondeclarationinstantiation`, step
    /// 28.f.i.4).
    BodyVar(FunctionId, u32),
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
    /// Looked up by name at run time, through the environments from the LexicalEnvironment
    /// outward: a `with` body lies between the reference and the scope that binds it.
    Dynamic(String),
    /// A `let` or `const` binding named `name`, accessed by an instruction that checks its
    /// state (`checked`), or a `const` one, which every assignment throws for. A declaration
    /// initializes its binding through the plain place, and an unchecked access to a `let`
    /// binding is a [`Binding::Local`] or [`Binding::Env`].
    Lexical {
        at: LexicalPlace,
        name: String,
        checked: bool,
        constant: bool,
    },
}

/// What the compiler needs of a function beyond its code.
#[derive(Clone, Debug, Default)]
pub(crate) struct FunctionInfo {
    pub locals: u32,
    /// The names of the function environment's slots, in slot order.
    pub captured: Vec<String>,
    pub params: Vec<Slot>,
    pub self_binding: bool,
    /// The arguments object a call makes, if any.
    pub arguments: Option<ArgumentsObject>,
    /// The name NamedEvaluation or an accessor gives an anonymous function.
    pub given_name: Option<Vec<u16>>,
    /// Of a function whose parameters have an initializer, when the code makes a record for the
    /// `var` scope of its body: the names of its slots, in slot order (empty when only the
    /// bindings a direct `eval` adds will live there).
    pub var_env: Option<Vec<String>>,
    /// Of a generator function: the local slot its code reserves for its generator object, the
    /// last of `locals` ("Suspended frames", item 2, `docs/design/engine.md`).
    pub generator_slot: Option<u32>,
    /// Of an `async` function or arrow function: the local slot its code reserves for its
    /// activation, the last of `locals` ("`async` functions on suspended frames", item 1,
    /// `docs/design/engine.md`).
    pub async_slot: Option<u32>,
}

pub(crate) struct Resolution {
    pub sites: BTreeMap<Site, Binding>,
    pub script_locals: u32,
    /// Indexed by `FunctionId`.
    pub functions: Vec<FunctionInfo>,
    /// The try statements whose `catch` clause has an environment with named slots.
    pub named_catches: BTreeSet<StmtId>,
    /// The lexical scopes with a record or a checked local slot.
    pub lexical_scopes: BTreeMap<ScopeKey, LexicalScope>,
    /// Of eval code: the lexical names of its top level in source order, each with whether it
    /// is `const` and its slot.
    pub eval_lexical: Vec<(String, bool, Slot)>,
    /// Of eval code: the names of the slots of the record its lexical bindings need, in slot
    /// order, with whether each is `const`; empty when none needs one.
    pub eval_env: Vec<(String, bool)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// The parameters, `arguments` and, when no parameter has an initializer, the `var` names
    /// of a function. The parameters of a list that is not simple are bound in order, each
    /// uninitialized until its turn (`#sec-functiondeclarationinstantiation`, steps 21 and 26).
    Function,
    /// The `var` names of the body of a function whose parameters have an initializer, inside
    /// the scope of its parameters (`#sec-functiondeclarationinstantiation`, step 28).
    Vars,
    Catch,
    SelfName,
    /// The body of a `with` statement: no binding, an Object Environment Record at run time.
    With,
    /// A scope of `let` and `const` bindings. In a `straight` one (a block, a function body,
    /// eval code) the code after a declaration in the same function runs after it; in a case
    /// block, a head or an iteration every access is checked.
    Lexical {
        straight: bool,
    },
}

/// A `let` or `const` binding: whether it is `const`, and the end of its declaration.
#[derive(Clone, Copy)]
struct LexicalData {
    constant: bool,
    end: u32,
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
    /// Whether its environment keeps the names of its slots: it encloses a `with` body.
    named: bool,
    /// The try statement of a `catch` scope.
    catch: Option<StmtId>,
    /// The node of a lexical scope; `None` for the top level of eval code and other scopes.
    key: Option<ScopeKey>,
}

struct BindingData {
    scope: usize,
    name: String,
    captured: bool,
    place: Place,
    lexical: Option<LexicalData>,
}

#[derive(Clone, Copy)]
enum Walk {
    Stmt(StmtId),
    Expr(ExprId),
    Function(FunctionId),
    /// Enters the body of a function, after the initializers of its parameters.
    Body(FunctionId),
    Catch(StmtId),
    /// Enters the body of the `with` statement.
    With,
    /// Enters the lexical scope of the node, when it declares a name; a [`Walk::Leave`] follows
    /// its statements only then, which [`Walk::LeaveLexical`] is.
    Lexical(ScopeKey),
    /// Leaves the lexical scope of the node, if it was entered.
    LeaveLexical(ScopeKey),
    Leave,
}

impl HeapBytes for Walk {
    fn heap_bytes(&self) -> usize {
        0
    }
}

struct Resolver<'a, 'm> {
    ast: &'a Ast,
    /// The compile step's ledger: every growth of the resolver's structures is charged to it
    /// first; the compile step credits them all when it ends.
    m: &'a Meter<'m>,
    scopes: Vec<ScopeData>,
    bindings: Vec<BindingData>,
    current: usize,
    /// Each site with the scope it is resolved from and its name.
    occurrences: Vec<(Site, usize, String)>,
    given_names: BTreeMap<FunctionId, Vec<u16>>,
    /// The scope of the own name of each named function expression.
    self_scopes: BTreeMap<FunctionId, usize>,
    /// The scope of the parameters and `var` declarations of each function.
    function_scopes: BTreeMap<FunctionId, usize>,
    /// The scope of the `var` names of the body of each function whose parameters have an
    /// initializer (`Kind::Vars`).
    vars_scopes: BTreeMap<FunctionId, usize>,
    /// The scope of each direct `eval` call.
    eval_calls: Vec<usize>,
    /// Whether the tree is eval code: a name it does not bind is looked up by name.
    eval_code: bool,
    /// Whether the script's own code is non-strict and contains a direct `eval` call.
    script_adds_bindings: bool,
    work: Vec<Walk>,
    /// The `let` and `const` bindings in local slots that an access checks, by binding index.
    checked_locals: BTreeSet<usize>,
}

fn malformed(what: &str) -> CompileError {
    CompileError::Tree(what.to_string())
}

/// Resolves every name of `script`; of eval code when `eval_code` is true.
pub(crate) fn resolve(
    script: &Script,
    eval_code: bool,
    m: &Meter<'_>,
) -> Result<Resolution, CompileError> {
    let mut scopes = m.with_capacity(1)?;
    scopes.push(ScopeData {
        parent: None,
        kind: Kind::Function,
        owner: None,
        names: BTreeMap::new(),
        order: Vec::new(),
        env_slots: 0,
        named: false,
        catch: None,
        key: None,
    });
    let mut r = Resolver {
        ast: &script.ast,
        m,
        scopes,
        bindings: Vec::new(),
        current: 0,
        occurrences: Vec::new(),
        given_names: BTreeMap::new(),
        self_scopes: BTreeMap::new(),
        function_scopes: BTreeMap::new(),
        vars_scopes: BTreeMap::new(),
        eval_calls: Vec::new(),
        eval_code,
        script_adds_bindings: !script.strict && script.scope.contains_direct_eval,
        work: Vec::new(),
        checked_locals: BTreeSet::new(),
    };
    let resolution = resolve_with(&mut r, script, eval_code);
    // The resolver's own structures are dropped with it; on a stop the compile step credits
    // everything at once.
    if resolution.is_ok() {
        m.credit(r.scratch_bytes());
    }
    drop(r);
    resolution
}

/// The steps of [`resolve`] with the resolver made.
fn resolve_with(
    r: &mut Resolver<'_, '_>,
    script: &Script,
    eval_code: bool,
) -> Result<Resolution, CompileError> {
    // The lexical names of eval code are a scope of its own (`#sec-evaldeclarationinstantiation`,
    // step 16); a script's are names of the global environment.
    let eval_top = if eval_code && !script.scope.lexical.is_empty() {
        let scope = r.new_scope(Kind::Lexical { straight: true }, None);
        r.declare_lexical(scope, &script.scope.lexical);
        Some(scope)
    } else {
        None
    };
    r.push_list(&script.body);
    while let Some(task) = r.work.pop() {
        r.m.visit()?;
        r.reserve_work(task)?;
        let reserved = r.work.capacity();
        r.step(task)?;
        r.settle_work(reserved)?;
        r.m.check()?;
    }
    r.mark_captured()?;
    r.mark_named()?;
    let mut frames = r.allocate()?;
    let sites = r.locate()?;
    let lexical_scopes = r.lexical_scopes()?;
    let (eval_lexical, eval_env) = match eval_top {
        Some(scope) => r.eval_bindings(scope)?,
        None => (Vec::new(), Vec::new()),
    };
    let script_locals = frames.remove(&None).unwrap_or(0);
    let mut functions = r.m.with_capacity(script.ast.functions().len())?;
    for (i, _) in script.ast.functions().iter().enumerate() {
        let id = FunctionId(u32::try_from(i).map_err(|_| CompileError::TooMany("functions"))?);
        let mut info = r.function_info(id, &sites)?;
        info.locals = frames.get(&Some(id)).copied().unwrap_or(0);
        if script.ast.function(id).is_some_and(|f| f.generator) {
            info.generator_slot = Some(info.locals);
            info.locals = info
                .locals
                .checked_add(1)
                .ok_or(CompileError::TooMany("local slots"))?;
        }
        if script.ast.function(id).is_some_and(|f| f.asynchronous) {
            info.async_slot = Some(info.locals);
            info.locals = info
                .locals
                .checked_add(1)
                .ok_or(CompileError::TooMany("local slots"))?;
        }
        info.given_name = r.given_names.remove(&id);
        functions.push(info);
    }
    let mut named_catches = BTreeSet::new();
    for catch in r.scopes.iter().filter(|s| s.named).filter_map(|s| s.catch) {
        r.m.insert(&mut named_catches, catch)?;
    }
    Ok(Resolution {
        sites,
        script_locals,
        functions,
        named_catches,
        lexical_scopes,
        eval_lexical,
        eval_env,
    })
}

impl<'a, 'm> Resolver<'a, 'm> {
    /// Room on the work stack for what the step `task` pushes, charged before the step: its
    /// node's children twice over and ten more (a step pushes its children and at most a few
    /// markers around them).
    fn reserve_work(&mut self, task: Walk) -> Result<(), Stop> {
        let children = match task {
            Walk::Stmt(id) => self.ast.stmt_child_count(id),
            Walk::Expr(id) => self.ast.expr_child_count(id),
            Walk::Function(id) => self
                .ast
                .function(id)
                .map_or(0, |f| f.initializers.len().saturating_add(f.params.len())),
            Walk::Body(id) => self.ast.function(id).map_or(0, |f| f.body.len()),
            _ => 0,
        };
        self.m.grow_by(
            &mut self.work,
            children.saturating_mul(2).saturating_add(10),
        )
    }

    /// Charges what the work stack grew past `reserved`, its capacity before the step, if the
    /// step pushed more than it reserved (an upper bound: a list pushed by `push_list` charged
    /// its own growth already).
    fn settle_work(&mut self, reserved: usize) -> Result<(), Stop> {
        let now = self.work.capacity();
        if now > reserved {
            self.m.charge(
                now.saturating_sub(reserved)
                    .saturating_mul(size_of::<Walk>()),
            )?;
        }
        Ok(())
    }

    /// What the resolver's own structures hold charged, at most: the work stack, the
    /// occurrences, the bindings and the scopes with their names, the direct `eval` calls, the
    /// entries of the maps of function scopes and of the names left to give, and the bound
    /// charged for `checked_locals` (one entry per binding). Each vector was charged by its
    /// capacity and each name by at least its length.
    fn scratch_bytes(&self) -> usize {
        fn capacity<T>(v: &Vec<T>) -> usize {
            v.capacity().saturating_mul(size_of::<T>())
        }
        let occurrences = self
            .occurrences
            .iter()
            .fold(capacity(&self.occurrences), |sum, (_, _, name)| {
                sum.saturating_add(name.len())
            });
        let bindings = self
            .bindings
            .iter()
            .fold(capacity(&self.bindings), |sum, b| {
                sum.saturating_add(b.name.len())
            });
        let scopes = self.scopes.iter().fold(capacity(&self.scopes), |sum, s| {
            s.names
                .keys()
                .fold(sum, |sum, key| sum.saturating_add(key.len()))
                .saturating_add(
                    s.names
                        .len()
                        .saturating_mul(map_entry_bytes::<String, usize>()),
                )
                .saturating_add(capacity(&s.order))
        });
        let given = self
            .given_names
            .values()
            .fold(0usize, |sum, name| sum.saturating_add(capacity(name)))
            .saturating_add(
                self.given_names
                    .len()
                    .saturating_mul(map_entry_bytes::<FunctionId, Vec<u16>>()),
            );
        let function_maps = self
            .self_scopes
            .len()
            .saturating_add(self.function_scopes.len())
            .saturating_add(self.vars_scopes.len())
            .saturating_mul(map_entry_bytes::<FunctionId, usize>());
        let checked = self
            .bindings
            .len()
            .saturating_mul(map_entry_bytes::<usize, ()>());
        capacity(&self.work)
            .saturating_add(occurrences)
            .saturating_add(bindings)
            .saturating_add(scopes)
            .saturating_add(capacity(&self.eval_calls))
            .saturating_add(given)
            .saturating_add(function_maps)
            .saturating_add(checked)
    }

    fn push_list(&mut self, list: &[StmtId]) {
        if self.m.grow_by(&mut self.work, list.len()).is_err() {
            return;
        }
        for s in list.iter().rev() {
            self.work.push(Walk::Stmt(*s));
        }
    }

    /// Pushes the statement list `list` inside the lexical scope of `key`.
    fn push_lexical_list(&mut self, key: ScopeKey, list: &[StmtId]) {
        if self
            .m
            .grow_by(&mut self.work, list.len().saturating_add(2))
            .is_err()
        {
            return;
        }
        self.work.push(Walk::LeaveLexical(key));
        self.push_list(list);
        self.work.push(Walk::Lexical(key));
    }

    /// A new scope inside the current one, made current. When its growth is refused the meter
    /// has stopped, nothing is pushed, and the step ends at its next check.
    fn new_scope(&mut self, kind: Kind, owner: Option<FunctionId>) -> usize {
        if self.m.grow(&mut self.scopes).is_err() {
            return self.current;
        }
        self.scopes.push(ScopeData {
            parent: Some(self.current),
            kind,
            owner,
            names: BTreeMap::new(),
            order: Vec::new(),
            env_slots: 0,
            named: false,
            catch: None,
            key: None,
        });
        self.current = self.scopes.len() - 1;
        self.current
    }

    /// Declares `name` in `scope` once.
    fn declare(&mut self, scope: usize, name: &str) {
        self.declare_as(scope, name, None);
    }

    /// Declares `name` in `scope` once, a `let` or `const` binding when `lexical` says so.
    fn declare_as(&mut self, scope: usize, name: &str, lexical: Option<LexicalData>) {
        let index = self.bindings.len();
        let Some(s) = self.scopes.get_mut(scope) else {
            return;
        };
        if s.names.contains_key(name) {
            return;
        }
        // The entry, the two copies of the name and the growth of the lists, charged first; a
        // refusal stops the meter and declares nothing.
        let m = self.m;
        if m.charge(map_entry_bytes::<String, usize>()).is_err() {
            return;
        }
        let (Ok(key), Ok(copy)) = (m.string(name), m.string(name)) else {
            return;
        };
        if m.grow(&mut s.order).is_err() || m.grow(&mut self.bindings).is_err() {
            return;
        }
        s.names.insert(key, index);
        s.order.push(index);
        self.bindings.push(BindingData {
            scope,
            name: copy,
            captured: false,
            place: Place::Unset,
            lexical,
        });
    }

    /// Declares the `let` and `const` names `names` in `scope`, each ending with its
    /// declaration statement. A function declaration among them (of a block or a `case` block)
    /// is a binding no access checks, initialized at the entry of the scope through its site
    /// [`Site::Declaration`], sited in `scope`, the current scope (decision 3, "Function
    /// declarations in a block", items 1 and 2).
    fn declare_lexical(&mut self, scope: usize, names: &[LexicalName]) {
        for n in names {
            let declaration = self.ast.stmt(n.declaration);
            if let Some(Stmt::FunctionDeclaration(f)) = declaration.map(|s| &s.stmt) {
                self.declare_as(scope, &n.name.name, None);
                self.occur(Site::Declaration(*f), &n.name.name);
                continue;
            }
            let end = declaration.map_or(u32::MAX, |s| s.span.end);
            let data = LexicalData {
                constant: n.constant,
                end,
            };
            self.declare_as(scope, &n.name.name, Some(data));
        }
    }

    /// The names the lexical scope of `key` declares, and whether it is a straight one.
    fn lexical_names(&self, key: ScopeKey) -> (Vec<LexicalName>, bool) {
        let ast = self.ast;
        let stmt = |id: StmtId| ast.stmt(id).map(|n| &n.stmt);
        let head = |id: StmtId, kind: DeclKind, names: Vec<&swell_syntax::ast::Name>| {
            names
                .into_iter()
                .map(|name| LexicalName {
                    name: name.clone(),
                    constant: kind == DeclKind::Const,
                    declaration: id,
                })
                .collect::<Vec<_>>()
        };
        match key {
            ScopeKey::Block(id) => match stmt(id) {
                Some(Stmt::Block(list)) => (ast.lexical_names(list), true),
                _ => (Vec::new(), true),
            },
            ScopeKey::Case(id) => match stmt(id) {
                Some(Stmt::Switch { cases, .. }) => (
                    cases
                        .iter()
                        .flat_map(|c| ast.lexical_names(&c.body))
                        .collect(),
                    false,
                ),
                _ => (Vec::new(), false),
            },
            ScopeKey::Head(id) => match stmt(id) {
                Some(Stmt::For {
                    init: Some(ForInit::Var(kind, declarators)),
                    ..
                }) if kind.is_lexical() => (
                    head(
                        id,
                        *kind,
                        declarators.iter().flat_map(|d| d.target.names()).collect(),
                    ),
                    false,
                ),
                _ => (Vec::new(), false),
            },
            ScopeKey::Tdz(id) | ScopeKey::Iteration(id) => match stmt(id) {
                Some(Stmt::ForIn {
                    target: ForInTarget::Var(kind, target),
                    ..
                }) if kind.is_lexical() => (head(id, *kind, target.names().collect()), false),
                _ => (Vec::new(), false),
            },
            // Entered by `Resolver::catch`, with the end of the pattern as each name's end.
            ScopeKey::CatchParameter(_) => (Vec::new(), true),
            ScopeKey::Try(id) | ScopeKey::Catch(id) | ScopeKey::Finally(id) => {
                let list = match (key, stmt(id)) {
                    (ScopeKey::Try(_), Some(Stmt::Try { block, .. })) => Some(block.as_slice()),
                    (
                        ScopeKey::Catch(_),
                        Some(Stmt::Try {
                            handler: Some(h), ..
                        }),
                    ) => Some(h.body.as_slice()),
                    (
                        ScopeKey::Finally(_),
                        Some(Stmt::Try {
                            finalizer: Some(f), ..
                        }),
                    ) => Some(f.as_slice()),
                    _ => None,
                };
                (list.map(|l| ast.lexical_names(l)).unwrap_or_default(), true)
            }
            ScopeKey::Body(f) => (
                ast.function(f)
                    .map(|f| f.scope.lexical.clone())
                    .unwrap_or_default(),
                true,
            ),
        }
    }

    /// Enters the lexical scope of `key` when it declares a name: a for-in iteration's binding
    /// is initialized by the key (`Site::ForInVar`), or by the pattern of the head.
    fn enter_lexical(&mut self, key: ScopeKey) {
        let (names, straight) = self.lexical_names(key);
        if names.is_empty() {
            return;
        }
        // The list of the scope's names is scratch of this step: charged by what it owns, as
        // it is made by `Ast::lexical_names`, and credited when the step ends.
        let owned = names.heap_bytes();
        if self.m.charge(owned).is_err() {
            return;
        }
        self.enter_lexical_names(key, &names, straight);
        drop(names);
        self.m.credit(owned);
    }

    /// The scope of `key` with the lexical names `names`, entered.
    fn enter_lexical_names(&mut self, key: ScopeKey, names: &[LexicalName], straight: bool) {
        let owner = self.scopes.get(self.current).and_then(|s| s.owner);
        let scope = self.new_scope(Kind::Lexical { straight }, owner);
        if let Some(s) = self.scopes.get_mut(scope) {
            s.key = Some(key);
        }
        self.declare_lexical(scope, names);
        if let ScopeKey::Iteration(id) = key
            && let Some(n) = names.first()
            && self.for_in_identifier(id)
        {
            self.occur(Site::ForInVar(id), &n.name.name);
        }
    }

    /// Whether the head of the for-in statement `id` declares a `BindingIdentifier`, not a
    /// pattern.
    fn for_in_identifier(&self, id: StmtId) -> bool {
        matches!(
            self.ast.stmt(id).map(|n| &n.stmt),
            Some(Stmt::ForIn {
                target: ForInTarget::Var(_, BindingTarget::Identifier(_)),
                ..
            })
        )
    }

    /// The site `site` of `name` in the current scope; its copy of the name and the growth of
    /// the list charged first (a refusal stops the meter and records nothing).
    fn occur(&mut self, site: Site, name: &str) {
        let Ok(copy) = self.m.string(name) else {
            return;
        };
        let current = self.current;
        let _ = self.m.push(&mut self.occurrences, (site, current, copy));
    }

    fn step(&mut self, task: Walk) -> Result<(), CompileError> {
        match task {
            Walk::Stmt(id) => self.stmt(id),
            Walk::Expr(id) => self.expr(id),
            Walk::Function(id) => self.function(id),
            Walk::Body(id) => self.function_body(id),
            Walk::Catch(id) => self.catch(id),
            Walk::With => {
                let owner = self.scopes.get(self.current).and_then(|s| s.owner);
                self.new_scope(Kind::With, owner);
                Ok(())
            }
            Walk::Lexical(key) => {
                self.enter_lexical(key);
                Ok(())
            }
            Walk::LeaveLexical(key) => {
                let here = self.scopes.get(self.current);
                if here.and_then(|s| s.key) == Some(key) {
                    self.current = here
                        .and_then(|s| s.parent)
                        .ok_or_else(|| malformed("a lexical scope left twice"))?;
                }
                Ok(())
            }
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
        anonymous_function(self.ast, e)
    }

    fn stmt(&mut self, id: StmtId) -> Result<(), CompileError> {
        let node = self
            .ast
            .stmt(id)
            .ok_or_else(|| malformed("a statement index"))?;
        match &node.stmt {
            Stmt::Block(list) => {
                self.work.push(Walk::LeaveLexical(ScopeKey::Block(id)));
                self.push_list(list);
                self.work.push(Walk::Lexical(ScopeKey::Block(id)));
            }
            Stmt::Var(_, declarators)
            | Stmt::For {
                init: Some(ForInit::Var(_, declarators)),
                ..
            } => {
                // The head of a `for` with `let` or `const` is a scope around the whole loop.
                let head = matches!(
                    &node.stmt,
                    Stmt::For {
                        init: Some(ForInit::Var(kind, _)),
                        ..
                    } if kind.is_lexical()
                );
                if head {
                    self.enter_lexical(ScopeKey::Head(id));
                }
                // Room for every child pushed below, charged first: an initializer and a
                // pattern per declarator, and four more.
                let mut children = self
                    .m
                    .with_capacity(declarators.len().saturating_mul(2).saturating_add(4))?;
                for (i, d) in declarators.iter().enumerate() {
                    let n = u32::try_from(i).map_err(|_| CompileError::TooMany("declarators"))?;
                    let name = d.target.identifier();
                    if let Some(name) = name {
                        self.occur(Site::VarDecl(id, n), &name.name);
                    }
                    if let Some(init) = d.init {
                        if let Some(name) = name
                            && let Some(f) = self.anonymous_function(init)
                        {
                            let name = self.m.utf16(&name.name)?;
                            self.m.charge(map_entry_bytes::<FunctionId, Vec<u16>>())?;
                            self.given_names.insert(f, name);
                        }
                        children.push(Walk::Expr(init));
                    }
                    // A pattern's names are bound after the initializer is evaluated.
                    children.extend(d.target.pattern().map(Walk::Expr));
                }
                if let Stmt::For {
                    test, update, body, ..
                } = &node.stmt
                {
                    children.extend(test.map(Walk::Expr));
                    children.extend(update.map(Walk::Expr));
                    children.push(Walk::Stmt(*body));
                }
                if head {
                    children.push(Walk::LeaveLexical(ScopeKey::Head(id)));
                }
                self.m.grow_by(&mut self.work, children.len())?;
                self.work.extend(children.drain(..).rev());
                self.m.release(children);
            }
            Stmt::Empty | Stmt::Continue(_) | Stmt::Break(_) | Stmt::Debugger => {}
            // A label binds nothing (`#sec-labelled-statements`).
            Stmt::Labelled { body, .. } => self.work.push(Walk::Stmt(*body)),
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
            // With `let` or `const`: the expression in a scope of the head's names, then each
            // iteration in a scope of its own (`#sec-runtime-semantics-forinofheadevaluation`,
            // `#sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset`).
            Stmt::ForIn {
                target: ForInTarget::Var(kind, target),
                object,
                body,
                ..
            } if kind.is_lexical() => {
                self.work.push(Walk::LeaveLexical(ScopeKey::Iteration(id)));
                self.work.push(Walk::Stmt(*body));
                // A pattern binds the iteration's names, inside its scope.
                self.work.extend(target.pattern().map(Walk::Expr));
                self.work.push(Walk::Lexical(ScopeKey::Iteration(id)));
                self.work.push(Walk::LeaveLexical(ScopeKey::Tdz(id)));
                self.work.push(Walk::Expr(*object));
                self.work.push(Walk::Lexical(ScopeKey::Tdz(id)));
            }
            Stmt::ForIn {
                target,
                object,
                body,
                ..
            } => {
                self.work.push(Walk::Stmt(*body));
                self.work.push(Walk::Expr(*object));
                match target {
                    ForInTarget::Var(_, BindingTarget::Identifier(name)) => {
                        self.occur(Site::ForInVar(id), &name.name)
                    }
                    ForInTarget::Var(_, BindingTarget::Pattern(p)) => {
                        self.work.push(Walk::Expr(p.pattern))
                    }
                    ForInTarget::Expression(e) => self.work.push(Walk::Expr(*e)),
                }
            }
            // The case block is a scope entered after the discriminant.
            Stmt::Switch {
                discriminant,
                cases,
            } => {
                self.work.push(Walk::LeaveLexical(ScopeKey::Case(id)));
                for case in cases.iter().rev() {
                    self.push_list(&case.body);
                    self.work.extend(case.test.map(Walk::Expr));
                }
                self.work.push(Walk::Lexical(ScopeKey::Case(id)));
                self.work.push(Walk::Expr(*discriminant));
            }
            Stmt::Try {
                block,
                handler,
                finalizer,
            } => {
                if let Some(list) = finalizer {
                    self.push_lexical_list(ScopeKey::Finally(id), list);
                }
                // A `catch` without a parameter makes no scope
                // (`#sec-runtime-semantics-catchclauseevaluation`, `Catch : catch Block`); its
                // block is a scope inside the parameter's.
                // A pattern is a scope of its names, uninitialized until the pattern binds them.
                if let Some(h) = handler {
                    match &h.param {
                        Some(BindingTarget::Identifier(_)) => {
                            self.work.push(Walk::Leave);
                            self.push_lexical_list(ScopeKey::Catch(id), &h.body);
                            self.work.push(Walk::Catch(id));
                        }
                        Some(BindingTarget::Pattern(p)) => {
                            self.work
                                .push(Walk::LeaveLexical(ScopeKey::CatchParameter(id)));
                            self.push_lexical_list(ScopeKey::Catch(id), &h.body);
                            self.work.push(Walk::Expr(p.pattern));
                            self.work.push(Walk::Catch(id));
                        }
                        None => self.push_lexical_list(ScopeKey::Catch(id), &h.body),
                    }
                }
                self.push_lexical_list(ScopeKey::Try(id), block);
            }
            Stmt::FunctionDeclaration(f) => self.work.push(Walk::Function(*f)),
            // The object is evaluated outside the scope of the body.
            Stmt::With { object, body } => {
                self.work.push(Walk::Leave);
                self.work.push(Walk::Stmt(*body));
                self.work.push(Walk::With);
                self.work.push(Walk::Expr(*object));
            }
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
        let param = h
            .param
            .as_ref()
            .ok_or_else(|| malformed("a catch clause without a parameter"))?;
        let owner = self.scopes.get(self.current).and_then(|s| s.owner);
        let p = match param {
            BindingTarget::Identifier(name) => {
                let scope = self.new_scope(Kind::Catch, owner);
                if let Some(s) = self.scopes.get_mut(scope) {
                    s.catch = Some(id);
                }
                self.declare(scope, &name.name);
                self.occur(Site::CatchParam(id), &name.name);
                return Ok(());
            }
            BindingTarget::Pattern(p) => p,
        };
        // A pattern's names are a scope of `let`-like bindings: an access in the pattern itself
        // is checked, one in the block after it is not.
        let key = ScopeKey::CatchParameter(id);
        let scope = self.new_scope(Kind::Lexical { straight: true }, owner);
        if let Some(s) = self.scopes.get_mut(scope) {
            s.key = Some(key);
        }
        let data = LexicalData {
            constant: false,
            end: p.span.end,
        };
        for b in &p.names {
            self.declare_as(scope, &b.name.name, Some(data));
        }
        Ok(())
    }

    fn expr(&mut self, id: ExprId) -> Result<(), CompileError> {
        let node = self
            .ast
            .expr(id)
            .ok_or_else(|| malformed("an expression index"))?;
        // Room for the node's children, charged first and credited when they are on the work
        // stack.
        let mut children: Vec<Walk> = self.m.with_capacity(self.ast.expr_child_count(id))?;
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
                    // A computed key is evaluated before its value, and its function gets its
                    // name when the literal is evaluated (`#sec-setfunctionname`).
                    let computed = match p.key {
                        PropertyKey::Computed(k) => {
                            children.push(Walk::Expr(k));
                            true
                        }
                        _ => false,
                    };
                    match p.value {
                        PropertyValue::Init(e) => {
                            if let Some(f) = self.anonymous_function(e)
                                && !is_proto(&p.key)
                                && !computed
                            {
                                let key = property_key(&p.key, self.m)?;
                                self.give_name(f, key)?;
                            }
                            children.push(Walk::Expr(e));
                        }
                        PropertyValue::Get(f) | PropertyValue::Set(f) => {
                            if computed {
                                children.push(Walk::Function(f));
                                continue;
                            }
                            let prefix = if matches!(p.value, PropertyValue::Get(_)) {
                                "get "
                            } else {
                                "set "
                            };
                            let key = property_key(&p.key, self.m)?;
                            let mut name = self.m.with_capacity(prefix.len() + key.len())?;
                            name.extend(prefix.encode_utf16());
                            name.extend_from_slice(&key);
                            self.m.release(key);
                            self.give_name(f, name)?;
                            children.push(Walk::Function(f));
                        }
                        // DefineMethod gives a method its key as name
                        // (`#sec-runtime-semantics-definemethod`, SetFunctionName in
                        // DefineMethodProperty's caller).
                        PropertyValue::Method(f) => {
                            if !computed {
                                let key = property_key(&p.key, self.m)?;
                                self.give_name(f, key)?;
                            }
                            children.push(Walk::Function(f));
                        }
                        PropertyValue::Shorthand(e) | PropertyValue::Spread(e) => {
                            children.push(Walk::Expr(e));
                        }
                        // The parser rereads such a literal as a pattern, or refuses it.
                        PropertyValue::CoverInitialized(..) => {
                            return Err(malformed("a CoverInitializedName in an object literal"));
                        }
                    }
                }
            }
            // `#sec-runtime-semantics-keyedbindinginitialization` and
            // `#sec-runtime-semantics-keyeddestructuringassignmentevaluation`: each name a
            // binding pattern binds is a site; a default gives an anonymous function the name of
            // a `BindingIdentifier` or of an identifier target that is not parenthesized
            // (NamedEvaluation).
            Expr::ObjectPattern { properties, .. } => {
                for (i, p) in properties.iter().enumerate() {
                    let n = u32::try_from(i).map_err(|_| CompileError::TooMany("properties"))?;
                    if let PropertyKey::Computed(k) = p.key {
                        children.push(Walk::Expr(k));
                    }
                    let name = match &p.target {
                        PatternTarget::Binding(name) => {
                            self.occur(Site::Bound(id, n), &name.name);
                            Some(name.name.as_str())
                        }
                        PatternTarget::Expr(e) => {
                            children.push(Walk::Expr(*e));
                            match self.ast.expr(*e) {
                                Some(t) if !t.parenthesized => match &t.expr {
                                    Expr::Identifier(name) => Some(name.as_str()),
                                    _ => None,
                                },
                                _ => None,
                            }
                        }
                    };
                    let Some(default) = p.default else { continue };
                    if let Some(name) = name
                        && let Some(f) = self.anonymous_function(default)
                    {
                        let name = self.m.utf16(name)?;
                        self.give_name(f, name)?;
                    }
                    children.push(Walk::Expr(default));
                }
            }
            // `#sec-runtime-semantics-iteratorbindinginitialization` and
            // `#sec-runtime-semantics-iteratordestructuringassignmentevaluation`: as an object
            // pattern's, each element's name a site of its index, elisions counted.
            Expr::ArrayPattern { elements, .. } => {
                for (i, p) in elements.iter().enumerate() {
                    let Some(p) = p else { continue };
                    let n = u32::try_from(i).map_err(|_| CompileError::TooMany("elements"))?;
                    let name = match &p.target {
                        PatternTarget::Binding(name) => {
                            self.occur(Site::Bound(id, n), &name.name);
                            Some(name.name.as_str())
                        }
                        PatternTarget::Expr(e) => {
                            children.push(Walk::Expr(*e));
                            match self.ast.expr(*e) {
                                Some(t) if !t.parenthesized => match &t.expr {
                                    Expr::Identifier(name) => Some(name.as_str()),
                                    _ => None,
                                },
                                _ => None,
                            }
                        }
                    };
                    let Some(default) = p.default else { continue };
                    if let Some(name) = name
                        && let Some(f) = self.anonymous_function(default)
                    {
                        let name = self.m.utf16(name)?;
                        self.give_name(f, name)?;
                    }
                    children.push(Walk::Expr(default));
                }
            }
            Expr::Spread(e) | Expr::Await(e) => children.push(Walk::Expr(*e)),
            Expr::Yield { argument, .. } => {
                children.extend(argument.iter().map(|e| Walk::Expr(*e)))
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
                // the target IsIdentifierRef (not in parentheses) and the operator is `=` or a
                // logical assignment, not a compound one.
                if !matches!(op, swell_syntax::ast::AssignOp::Compound(_))
                    && let Some(t) = self.ast.expr(*target)
                    && !t.parenthesized
                    && let Expr::Identifier(name) = &t.expr
                    && let Some(f) = self.anonymous_function(*value)
                {
                    let name = self.m.utf16(name)?;
                    self.give_name(f, name)?;
                }
                children.push(Walk::Expr(*target));
                children.push(Walk::Expr(*value));
            }
            Expr::Sequence(list) => children.extend(list.iter().map(|e| Walk::Expr(*e))),
            Expr::Member { object, .. } | Expr::OptionalMember { object, .. } => {
                children.push(Walk::Expr(*object));
            }
            Expr::OptionalChain(e) => children.push(Walk::Expr(*e)),
            Expr::Index { object, index } | Expr::OptionalIndex { object, index } => {
                children.push(Walk::Expr(*object));
                children.push(Walk::Expr(*index));
            }
            Expr::Call { callee, arguments } | Expr::OptionalCall { callee, arguments } => {
                if matches!(node.expr, Expr::Call { .. }) && self.ast.is_direct_eval_callee(*callee)
                {
                    self.m.push(&mut self.eval_calls, self.current)?;
                }
                children.push(Walk::Expr(*callee));
                children.extend(arguments.iter().map(|e| Walk::Expr(*e)));
            }
            Expr::New { callee, arguments } => {
                children.push(Walk::Expr(*callee));
                children.extend(arguments.iter().flatten().map(|e| Walk::Expr(*e)));
            }
            // The substitutions of a template, whose names resolve as any expression's.
            Expr::Template { substitutions, .. } => {
                children.extend(substitutions.iter().map(|e| Walk::Expr(*e)));
            }
            // A tag and the substitutions of a tagged template resolve as a callee and its
            // arguments do; a tag `eval` is no direct eval (`#sec-function-calls`).
            Expr::TaggedTemplate {
                tag, substitutions, ..
            } => {
                children.push(Walk::Expr(*tag));
                children.extend(substitutions.iter().map(|e| Walk::Expr(*e)));
            }
        }
        let bytes = children.capacity().saturating_mul(size_of::<Walk>());
        self.work.extend(children.into_iter().rev());
        self.m.credit(bytes);
        Ok(())
    }

    /// Gives the anonymous function `f` the name `name`, whose units are charged already; the
    /// entry is charged first.
    fn give_name(&mut self, f: FunctionId, name: Vec<u16>) -> Result<(), Stop> {
        self.m.insert_entry(&mut self.given_names, f, name)
    }

    /// Enters a function: the scope of its own name (a named function expression), then the
    /// scope of its parameters, `arguments` and, when no parameter has an initializer, its `var`
    /// declarations; the initializers of the parameters are walked in that scope, each
    /// anonymous function among them named by its parameter (NamedEvaluation,
    /// `#sec-runtime-semantics-iteratorbindinginitialization`), then the body
    /// ([`Resolver::function_body`]).
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
            self.m.insert_entry(&mut self.self_scopes, id, scope)?;
            self.work.push(Walk::Leave);
        }
        let scope = self.new_scope(Kind::Function, Some(id));
        self.m.insert_entry(&mut self.function_scopes, id, scope)?;
        let simple = f.is_simple();
        for (i, p) in f.params.iter().enumerate() {
            // A parameter of a list that is not simple is initialized in order: an access
            // before the end of its own parameter, its initializer included, is checked.
            let lexical = (!simple).then(|| LexicalData {
                constant: false,
                end: f
                    .initializer(i)
                    .and_then(|e| self.ast.expr(e))
                    .map_or(p.span().end, |n| n.span.end),
            });
            // The names of a pattern are sites of the pattern itself (`Site::Bound`).
            for name in p.names() {
                self.declare_as(scope, &name.name, lexical);
            }
            if let Some(name) = p.identifier() {
                let n = u32::try_from(i).map_err(|_| CompileError::TooMany("parameters"))?;
                self.occur(Site::Param(id, n), &name.name);
            }
        }
        // `#sec-functiondeclarationinstantiation`, steps 22 and 27: the binding `arguments` after
        // the parameters, before the `var` names (a `var arguments` is the same binding when no
        // parameter has an initializer).
        if f.scope.arguments_object {
            self.declare(scope, "arguments");
            self.occur(Site::Arguments(id), "arguments");
        }
        if !f.has_parameter_expressions() {
            for name in &f.scope.var_names {
                self.declare(scope, name);
            }
        }
        self.work.push(Walk::Leave);
        self.work.push(Walk::Body(id));
        // Each parameter's initializer, then its pattern, in order.
        for (i, p) in f.params.iter().enumerate().rev() {
            self.work.extend(p.pattern().map(Walk::Expr));
            let Some(e) = f.initializer(i) else { continue };
            if let Some(name) = p.identifier()
                && let Some(g) = self.anonymous_function(e)
            {
                let name = self.m.utf16(&name.name)?;
                self.give_name(g, name)?;
            }
            self.work.push(Walk::Expr(e));
        }
        Ok(())
    }

    /// Enters the body of function `id`, in the scope of its parameters: when a parameter has
    /// an initializer, a scope of its own for its `var` names first
    /// (`#sec-functiondeclarationinstantiation`, step 28), each name a parameter or `arguments`
    /// also binds starting with its value ([`Site::BodyVar`]); then the scope of the top level
    /// of the body.
    fn function_body(&mut self, id: FunctionId) -> Result<(), CompileError> {
        let f = self
            .ast
            .function(id)
            .ok_or_else(|| malformed("a function index"))?;
        if f.has_parameter_expressions() {
            let scope = self.new_scope(Kind::Vars, Some(id));
            self.m.insert_entry(&mut self.vars_scopes, id, scope)?;
            for (j, name) in f.scope.var_names.iter().enumerate() {
                self.declare(scope, name);
                let copied = f.parameter_names().any(|p| p.name == *name)
                    || (f.scope.arguments_object && name == "arguments");
                if copied && !self.declares_function(f, name) {
                    let n = u32::try_from(j).map_err(|_| CompileError::TooMany("var names"))?;
                    self.occur(Site::BodyVar(id, n), name);
                }
            }
            self.work.push(Walk::Leave);
        }
        // Steps 30 to 35: the top level of the body is a scope of its own inside the function's,
        // in which the functions it declares are made.
        self.enter_lexical(ScopeKey::Body(id));
        self.work.push(Walk::LeaveLexical(ScopeKey::Body(id)));
        for d in &f.scope.functions {
            let name = self
                .ast
                .function(*d)
                .and_then(|x| x.name.as_ref())
                .ok_or_else(|| malformed("a function declaration without a name"))?;
            self.occur(Site::Declaration(*d), &name.name);
        }
        self.push_list(&f.body);
        Ok(())
    }

    /// Whether the body of `f` declares a function named `name`, whose instantiation sets the
    /// binding (step 28.f.i.3: such a `var` starts as `undefined`).
    fn declares_function(&self, f: &swell_syntax::ast::Function, name: &str) -> bool {
        f.scope.functions.iter().any(|d| {
            self.ast
                .function(*d)
                .and_then(|x| x.name.as_ref())
                .is_some_and(|n| n.name == name)
        })
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
                self.m.push(&mut marks, b)?;
            }
        }
        for b in marks {
            if let Some(binding) = self.bindings.get_mut(b) {
                binding.captured = true;
            }
        }
        Ok(())
    }

    /// Decision 3, item 3: every scope that encloses a `with` body or a direct `eval` call
    /// (strict or not), up to the global scope, keeps the names of its slots, and each of its
    /// bindings lives in its environment; the scope of the call itself is one of them. The
    /// parameters of a function that makes a mapped arguments object (a non-strict function
    /// whose parameter list is simple, `#sec-functiondeclarationinstantiation`, step 22) live in
    /// its function environment (`#sec-createmappedargumentsobject`: the map reads and writes
    /// their bindings).
    fn mark_named(&mut self) -> Result<(), CompileError> {
        let m = self.m;
        let mut named = BTreeSet::new();
        let withs = self.scopes.iter().filter(|s| s.kind == Kind::With).count();
        let mut starts: Vec<Option<usize>> =
            m.with_capacity(withs.saturating_add(self.eval_calls.len()))?;
        starts.extend(
            self.scopes
                .iter()
                .filter(|s| s.kind == Kind::With)
                .map(|s| s.parent)
                .chain(self.eval_calls.iter().map(|c| Some(*c))),
        );
        for start in starts {
            let mut outer = start;
            while let Some(p) = outer {
                m.visit()?;
                // Each scope is walked once: what encloses a named scope is named already.
                if !m.insert(&mut named, p)? {
                    break;
                }
                outer = self.scopes.get(p).and_then(|s| s.parent);
            }
        }
        let mut marks = Vec::new();
        for &scope in &named {
            let s = self
                .scopes
                .get_mut(scope)
                .ok_or_else(|| malformed("a scope"))?;
            s.named = true;
            m.grow_by(&mut marks, s.order.len())?;
            marks.extend(s.order.iter().copied());
        }
        for (id, scope) in &self.function_scopes {
            let mapped = self
                .ast
                .function(*id)
                .is_some_and(|f| f.scope.arguments_object && !f.strict && f.is_simple());
            if !mapped {
                continue;
            }
            let (Some(f), Some(s)) = (self.ast.function(*id), self.scopes.get(*scope)) else {
                return Err(malformed("a function scope"));
            };
            for p in f.parameter_names() {
                if let Some(b) = s.names.get(&p.name) {
                    m.push(&mut marks, *b)?;
                }
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
            let Some(s) = self.scopes.get(scope) else {
                continue;
            };
            let (owner, kind) = (s.owner, s.kind);
            if kind != Kind::SelfName && !frames.contains_key(&owner) {
                self.m
                    .charge(map_entry_bytes::<Option<FunctionId>, u32>())?;
                frames.insert(owner, 0);
            }
            let mut env_slots = 0u32;
            for i in 0..s.order.len() {
                let Some(b) = self.scopes.get(scope).and_then(|s| s.order.get(i).copied()) else {
                    continue;
                };
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

    /// Whether bindings may appear in `scope` at run time, so a reference crossing it is looked
    /// up by name: the script or a function whose own code is non-strict and contains a direct
    /// `eval` call (`#sec-performeval`, varEnv; `#sec-evaldeclarationinstantiation`), the scope of
    /// its parameters and that of its body's `var` names alike, and the top
    /// of eval code, whose names lie in the environments of the code that called `eval`.
    fn adds_bindings(&self, scope: &ScopeData) -> bool {
        if !matches!(scope.kind, Kind::Function | Kind::Vars) {
            return false;
        }
        match scope.owner {
            Some(id) => self
                .ast
                .function(id)
                .is_some_and(|f| !f.strict && f.scope.contains_direct_eval),
            None => self.eval_code || self.script_adds_bindings,
        }
    }

    /// The binding of a site that resolved to the `let` or `const` binding `b`, at `place` with
    /// `hops`, from `scope`: a declaration initializes it through its plain place; another
    /// access is checked unless it follows the declaration in the code of the same function in
    /// a straight scope (decision 3, "Lexical bindings", item 2). A checked local slot goes into
    /// `checked_locals`, for the scope's entry to set it uninitialized.
    fn lexical_site(
        &self,
        site: Site,
        scope: usize,
        b: usize,
        data: LexicalData,
        place: LexicalPlace,
        checked_locals: &mut BTreeSet<usize>,
    ) -> Result<Option<Binding>, CompileError> {
        if matches!(
            site,
            Site::VarDecl(..) | Site::ForInVar(_) | Site::Param(..) | Site::Bound(..)
        ) {
            return Ok(None);
        }
        let binding = self.bindings.get(b).ok_or_else(|| malformed("a binding"))?;
        let home = self
            .scopes
            .get(binding.scope)
            .ok_or_else(|| malformed("a scope"))?;
        let from = self.scopes.get(scope).ok_or_else(|| malformed("a scope"))?;
        let at = match site {
            Site::Expr(e) => self.ast.expr(e).map_or(0, |n| n.span.start),
            _ => 0,
        };
        let follows = matches!(home.kind, Kind::Lexical { straight: true } | Kind::Function)
            && home.owner == from.owner
            && at >= data.end;
        let checked = !follows;
        if checked && let LexicalPlace::Local(_) = place {
            checked_locals.insert(b);
        }
        if !checked && !data.constant {
            return Ok(None);
        }
        Ok(Some(Binding::Lexical {
            at: place,
            name: binding.name.clone(),
            checked,
            constant: data.constant,
        }))
    }

    /// What the entry of each lexical scope does: the names of its record's slots and the local
    /// slots it sets uninitialized; an iteration of a for-in initializes its binding at once,
    /// unless a pattern binds its names one after another.
    fn lexical_scopes(&self) -> Result<BTreeMap<ScopeKey, LexicalScope>, CompileError> {
        let m = self.m;
        let mut scopes = BTreeMap::new();
        for s in &self.scopes {
            let Some(key) = s.key else { continue };
            let at_once = matches!(key, ScopeKey::Iteration(id) if self.for_in_identifier(id));
            let mut scope = LexicalScope::default();
            for &b in &s.order {
                let Some(binding) = self.bindings.get(b) else {
                    continue;
                };
                let constant = binding.lexical.is_some_and(|l| l.constant);
                match binding.place {
                    Place::Env(_) => {
                        let name = m.string(&binding.name)?;
                        m.push(&mut scope.env, (name, constant))?;
                    }
                    Place::Local(n) if self.checked_locals.contains(&b) && !at_once => {
                        m.push(&mut scope.uninitialize, n)?;
                    }
                    _ => {}
                }
            }
            if !scope.env.is_empty() || !scope.uninitialize.is_empty() {
                m.charge(map_entry_bytes::<ScopeKey, LexicalScope>())?;
                scopes.insert(key, scope);
            }
        }
        Ok(scopes)
    }

    /// The lexical names of the top level of eval code, the scope `scope`, with their slots, and
    /// the names of the slots of its record.
    #[allow(clippy::type_complexity)]
    fn eval_bindings(
        &self,
        scope: usize,
    ) -> Result<(Vec<(String, bool, Slot)>, Vec<(String, bool)>), CompileError> {
        let m = self.m;
        let s = self.scopes.get(scope).ok_or_else(|| malformed("a scope"))?;
        let mut names = m.with_capacity(s.order.len())?;
        let mut env = Vec::new();
        for &b in &s.order {
            let binding = self.bindings.get(b).ok_or_else(|| malformed("a binding"))?;
            let constant = binding.lexical.is_some_and(|l| l.constant);
            let slot = match binding.place {
                Place::Local(n) => Slot::Local(n),
                Place::Env(n) => {
                    let name = m.string(&binding.name)?;
                    m.push(&mut env, (name, constant))?;
                    Slot::Env(n)
                }
                Place::Unset => return Err(malformed("a binding with no place")),
            };
            names.push((m.string(&binding.name)?, constant, slot));
        }
        Ok((names, env))
    }

    /// The binding of every site.
    fn locate(&mut self) -> Result<BTreeMap<Site, Binding>, CompileError> {
        // At most one entry per binding, charged first.
        self.m.charge(
            self.bindings
                .len()
                .saturating_mul(map_entry_bytes::<usize, ()>()),
        )?;
        let mut checked_locals = BTreeSet::new();
        let sites = self.sites(&mut checked_locals)?;
        self.checked_locals = checked_locals;
        Ok(sites)
    }

    /// The binding of every site; the checked local slots into `checked_locals`.
    fn sites(
        &self,
        checked_locals: &mut BTreeSet<usize>,
    ) -> Result<BTreeMap<Site, Binding>, CompileError> {
        let mut sites = BTreeMap::new();
        for (site, scope, name) in &self.occurrences {
            // The entry and the most its binding owns (a copy of the name), charged first.
            self.m.visit()?;
            self.m
                .charge(map_entry_bytes::<Site, Binding>().saturating_add(name.len()))?;
            let found = self.lookup(*scope, name);
            let home = match found {
                Some(b) => Some(
                    self.bindings
                        .get(b)
                        .ok_or_else(|| malformed("a binding"))?
                        .scope,
                ),
                None => None,
            };
            // The environments between the site and the scope that binds the name, or the
            // global scope; a `with` body among them, a function or script scope whose own code
            // is non-strict with a direct `eval`, or the top of eval code makes it a lookup by
            // name (decision 3, "Names", item 1).
            let mut hops = 0u32;
            let mut by_name = false;
            let mut s = *scope;
            while Some(s) != home {
                let here = self.scopes.get(s).ok_or_else(|| malformed("a scope"))?;
                if here.kind == Kind::With || self.adds_bindings(here) {
                    by_name = true;
                }
                if here.env_slots > 0 || here.kind == Kind::With {
                    hops = hops
                        .checked_add(1)
                        .ok_or(CompileError::TooMany("environment hops"))?;
                }
                match here.parent {
                    Some(parent) => s = parent,
                    None if home.is_none() => break,
                    None => return Err(malformed("a scope chain")),
                }
            }
            let binding = match found {
                _ if by_name => Binding::Dynamic(name.clone()),
                None => Binding::Global(name.clone()),
                Some(b) => {
                    let data = self.bindings.get(b).ok_or_else(|| malformed("a binding"))?;
                    let home = self
                        .scopes
                        .get(data.scope)
                        .ok_or_else(|| malformed("a scope"))?;
                    let (place, kind, lexical) = (data.place, home.kind, data.lexical);
                    let at = match place {
                        Place::Local(n) => Some(LexicalPlace::Local(n)),
                        Place::Env(slot) => Some(LexicalPlace::Env { hops, slot }),
                        Place::Unset => None,
                    };
                    let checked = match (lexical, at) {
                        (Some(l), Some(at)) => {
                            self.lexical_site(*site, *scope, b, l, at, checked_locals)?
                        }
                        _ => None,
                    };
                    match (checked, place, kind) {
                        (Some(binding), _, _) => binding,
                        (None, Place::Local(n), _) => Binding::Local(n),
                        (None, Place::Env(slot), Kind::SelfName) => Binding::Immutable {
                            hops,
                            slot,
                            name: data.name.clone(),
                        },
                        (None, Place::Env(slot), _) => Binding::Env { hops, slot },
                        (None, Place::Unset, _) => {
                            return Err(malformed("a binding with no place"));
                        }
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
        let m = self.m;
        let mut info = FunctionInfo::default();
        let Some(f) = self.ast.function(id) else {
            return Ok(info);
        };
        // A parameter's slot, or of a pattern the slot of each name it binds, in order.
        info.params = m.with_capacity(f.parameter_names().count())?;
        for (i, p) in f.params.iter().enumerate() {
            let n = u32::try_from(i).map_err(|_| CompileError::TooMany("parameters"))?;
            let names = match p {
                BindingTarget::Identifier(_) => &[][..],
                BindingTarget::Pattern(pattern) => pattern.names.as_slice(),
            };
            let one = p.identifier().map(|_| Site::Param(id, n));
            let sites_of_p = one
                .into_iter()
                .chain(names.iter().map(|b| Site::Bound(b.pattern, b.property)));
            for site in sites_of_p {
                let slot = match sites.get(&site) {
                    Some(Binding::Local(s)) => Slot::Local(*s),
                    Some(Binding::Env { slot, .. }) => Slot::Env(*slot),
                    _ => return Err(malformed("a parameter with no slot")),
                };
                info.params.push(slot);
            }
        }
        // The function's own scopes are found through their maps, not by a walk over every
        // scope of the tree: a walk per function made the step quadratic in the number of
        // functions.
        if let Some(s) = self
            .function_scopes
            .get(&id)
            .and_then(|s| self.scopes.get(*s))
        {
            let mut captured: Vec<(u32, String)> = Vec::new();
            for b in s.order.iter().filter_map(|b| self.bindings.get(*b)) {
                if let Place::Env(slot) = b.place {
                    let name = m.string(&b.name)?;
                    m.push(&mut captured, (slot, name))?;
                }
            }
            // The sort's scratch is at most the list itself.
            let scratch = captured.len().saturating_mul(size_of::<(u32, String)>());
            m.charge(scratch)?;
            captured.sort();
            m.credit(scratch);
            let mut names = m.with_capacity(captured.len())?;
            names.extend(captured.drain(..).map(|(_, n)| n));
            m.release(captured);
            m.release(std::mem::replace(&mut info.captured, names));
        }
        info.self_binding = self
            .self_scopes
            .get(&id)
            .and_then(|s| self.scopes.get(*s))
            .is_some_and(|s| s.env_slots > 0);
        // The record of the body's `var` scope: when a binding of it lives in one, or when a
        // direct `eval` of non-strict code may add bindings to it.
        if let Some(s) = self.vars_scopes.get(&id).and_then(|s| self.scopes.get(*s))
            && (s.env_slots > 0 || self.adds_bindings(s))
        {
            // `allocate` numbers the slots in the order of the scope's bindings.
            let mut names = m.with_capacity(s.env_slots as usize)?;
            for b in s.order.iter().filter_map(|b| self.bindings.get(*b)) {
                if let Place::Env(_) = b.place {
                    names.push(m.string(&b.name)?);
                }
            }
            info.var_env = Some(names);
        }
        if f.scope.arguments_object {
            let slot = match sites.get(&Site::Arguments(id)) {
                Some(Binding::Local(s)) => Slot::Local(*s),
                Some(Binding::Env { slot, .. }) => Slot::Env(*slot),
                _ => return Err(malformed("an arguments binding with no slot")),
            };
            info.arguments = Some(ArgumentsObject {
                mapped: !f.strict && f.is_simple(),
                slot,
            });
        }
        Ok(info)
    }
}

/// Whether a property name is `__proto__` (`#sec-object-initializer-runtime-semantics-propertydefinitionevaluation`):
/// an identifier or a string literal; a numeric key never is.
fn is_proto(key: &PropertyKey) -> bool {
    match key {
        PropertyKey::Identifier(s) => s == "__proto__",
        PropertyKey::String(s) => s.value.iter().copied().eq("__proto__".encode_utf16()),
        PropertyKey::Number(_) | PropertyKey::Computed(_) => false,
    }
}

/// The anonymous function definition `e` of `ast` is, if it is one
/// (`#sec-isanonymousfunctiondefinition`).
pub(crate) fn anonymous_function(ast: &Ast, e: ExprId) -> Option<FunctionId> {
    match ast.expr(e).map(|n| &n.expr) {
        Some(Expr::Function(f)) => ast
            .function(*f)
            .filter(|x| {
                x.name.is_none() && matches!(x.kind, FunctionKind::Expression | FunctionKind::Arrow)
            })
            .map(|_| *f),
        _ => None,
    }
}

/// The property key of a property name, as UTF-16 code units: an identifier's name, a string's
/// value, or a number's ToString (`#sec-object-initializer-runtime-semantics-evaluation`,
/// `LiteralPropertyName`).
/// The units are charged to `m` before they are allocated.
pub(crate) fn property_key(key: &PropertyKey, m: &Meter<'_>) -> Result<Vec<u16>, CompileError> {
    Ok(match key {
        PropertyKey::Identifier(s) => m.utf16(s)?,
        PropertyKey::String(s) => m.copy(&s.value)?,
        PropertyKey::Number(n) => {
            let value = numeric_literal(n.text.as_bytes())
                .ok_or_else(|| malformed("a numeric literal swell-num does not read"))?
                .value;
            // A Number's ToString has at most 25 characters (`#sec-numeric-types-number-tostring`).
            m.charge(32)?;
            let text = number_to_string(value);
            let units = m.utf16(&text);
            m.credit(32);
            units?
        }
        PropertyKey::Computed(_) => return Err(malformed("a computed key read as a literal one")),
    })
}
