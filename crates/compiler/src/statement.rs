//! Statements (clause 14, `#sec-ecmascript-language-statements-and-declarations`): block, `var`,
//! empty, expression, `if`, `do-while`, `while`, `for`, `for-in` and `for-of` with `var`, `let`,
//! `const` or a left-hand side, `continue`, `break`, `return`, `switch`, `throw`, `try` with
//! `catch` and `finally`, `debugger`, labelled statements with `break` and `continue` to a label,
//! and `with` in non-strict code (its body runs in an Object Environment Record; the names in it
//! are looked up at run time, `resolve.rs`). A declarator, a for-in or for-of head or a `catch`
//! parameter that is an object pattern destructures its value through the pattern node
//! (`expression.rs`); a `catch` pattern's names are a scope of their own
//! ([`ScopeKey::CatchParameter`]).
//!
//! # The iterator of a for-of
//!
//! A for-of keeps its Iterator Record on the operand stack for the whole loop
//! ([`Instruction::GetIterator`], [`Instruction::IteratorStep`]), and its IteratorClose
//! ([`Instruction::IteratorClose`]) is compiled like a `finally` block over the step and the
//! body (`docs/design/engine.md`, decision 3, "The iterator of `for`-`of`, of an array pattern
//! and of a spread"): a control entry of its own inside the loop's, entered with a completion
//! record by a throw, a `return`, and a `break` or `continue` that leaves the loop, and a
//! [`HandlerKind::Finally`] handler. A `continue` of the loop itself goes past it
//! ([`Control::Finally`]'s `closes`).
//!
//! # Labels
//!
//! LabelledEvaluation (`#sec-labelled-statements-runtime-semantics-labelledevaluation`) passes
//! the label set of a run of labels to the statement they label. A loop keeps it in its control
//! entry, so a `break` or `continue` with one of its labels is that loop's own
//! (`#sec-loopcontinues`); any other statement gets a control entry of its own, whose exit is
//! where a `break` with its label turns into a normal completion. Labels make no code.
//!
//! # Completion values
//!
//! Only a script's completion value is observable in slice 1 (ScriptEvaluation,
//! `#sec-runtime-semantics-scriptevaluation`; `eval` is slice 2's), so only script code keeps
//! it, in the frame's completion value register. An expression statement stores its value
//! there. A statement whose result is `UpdateEmpty(C, undefined)` (`#sec-updateempty`): `if`,
//! the loops (`V` starts as `undefined`, `#sec-loopcontinues`), `switch` and `try`, first stores
//! `undefined`, and so does a `catch` block when it is entered. A `finally` block keeps the
//! register of the completion it was entered with, then stores `undefined`, and puts the kept
//! value back when it completes normally (`#sec-try-statement-runtime-semantics-evaluation`: "If
//! finallyResult is a normal completion, set finallyResult to blockResult"); a `break` or
//! `continue` out of it leaves its own value or `undefined` (UpdateEmpty(finallyResult,
//! undefined)). The values of the other statements, and the way `break` and `continue` carry the
//! last value out of a loop, follow from the order the register is written in. Function code
//! drops the value of an expression statement.
//!
//! # Lexical scopes
//!
//! A scope of `let` and `const` bindings (`resolve.rs`) is entered by [`Compiler::enter_scope`]:
//! a record of its bindings that live in one ([`Instruction::PushLexicalEnv`]) and its checked
//! local slots set uninitialized ([`Instruction::Uninitialize`]); it is left by `PopEnv` when it
//! made a record, and by every `break`, `continue`, `return` and throw through it as a `catch`
//! clause's record is. The functions a block or a `case` block declares are made at its entry,
//! before its first statement ([`Compiler::block_functions`]). A declaration initializes its
//! binding: the store into the slot, or
//! [`Instruction::InitGlobal`] for a name of the script's top level
//! (`#sec-let-and-const-declarations-runtime-semantics-evaluation`). A `for (let ...)` whose
//! head has a record copies it before the first test and before each update
//! ([`Instruction::CopyEnv`], `#sec-forbodyevaluation`).

use swell_syntax::ast::{
    BindingTarget, DeclKind, Expr, ExprId, ForInTarget, ForInit, FunctionId, IterationKind, Span,
    Stmt, StmtId, VarDeclarator,
};

use crate::bytecode::{HandlerKind, Instruction, SlotName, TAG_NORMAL};
use crate::expression::Target;
use crate::resolve::{Binding, ScopeKey, Site};
use crate::{CompileError, Compiler, Constants, Control, JumpKind, List, Task, Tasks, malformed};
use swell_syntax::ledger::Meter;

/// The names of a record's slots, `let` or `const` ones.
pub(crate) fn slot_names(
    constants: &mut Constants,
    names: &[(String, bool)],
    m: &Meter<'_>,
) -> Result<Vec<SlotName>, CompileError> {
    let mut table = m.with_capacity(names.len())?;
    for (name, constant) in names {
        let k = constants.string_str(name, m)?;
        table.push(SlotName::lexical(k, *constant).ok_or(CompileError::TooMany("strings"))?);
    }
    Ok(table)
}

/// The exit of a lexical scope entered by [`Compiler::enter_scope`]: the record it made, if any,
/// is popped.
fn leave_scope(record: bool, span: Span, t: &mut Tasks<'_, '_>) -> Result<(), CompileError> {
    if record {
        t.push(Task::Emit(Instruction::PopEnv, span))?;
        t.push(Task::EnvsDown)?;
    }
    Ok(())
}

impl<'a, 'm> Compiler<'a, 'm> {
    /// The tasks of the statement `id`; a loop gets the label set `labels`.
    pub(crate) fn statement(
        &mut self,
        id: StmtId,
        mut labels: Vec<String>,
    ) -> Result<Tasks<'a, 'm>, CompileError> {
        let node = self
            .ast
            .stmt(id)
            .ok_or_else(|| malformed("a statement index"))?;
        let span = node.span;
        let mut t = Tasks::new(self.m);
        match &node.stmt {
            // `#sec-labelled-statements-runtime-semantics-labelledevaluation`: the labels of a run
            // of labelled statements go to the statement they label.
            Stmt::Labelled { .. } => {
                let mut item = id;
                while let Some(Stmt::Labelled { label, body }) =
                    self.ast.stmt(item).map(|n| &n.stmt)
                {
                    let name = self.m.string(&label.name)?;
                    self.m.push(&mut labels, name)?;
                    item = *body;
                }
                let is_loop = matches!(
                    self.ast.stmt(item).map(|n| &n.stmt),
                    Some(
                        Stmt::DoWhile { .. }
                            | Stmt::While { .. }
                            | Stmt::For { .. }
                            | Stmt::ForIn { .. }
                    )
                );
                if is_loop {
                    t.push(Task::LabelledLoop(item, labels))?;
                } else {
                    let exit = self.label();
                    t.push(Task::PushControl(Control::Label {
                        labels,
                        exit,
                        items: self.items,
                        envs: self.envs,
                    }))?;
                    t.push(Task::Stmt(item))?;
                    t.push(Task::PopControl(None))?;
                    t.push(Task::Bind(exit))?;
                }
            }
            // `#sec-block-runtime-semantics-evaluation`, with BlockDeclarationInstantiation.
            Stmt::Block(list) => {
                let record = self.enter_scope(ScopeKey::Block(id), span, &mut t)?;
                self.block_functions(list, &mut t)?;
                t.extend(list.iter().map(|s| Task::Stmt(*s)))?;
                leave_scope(record, span, &mut t)?;
            }
            // `#sec-let-and-const-declarations-runtime-semantics-evaluation`.
            Stmt::Var(kind, declarators) if kind.is_lexical() => {
                self.lexical_declarators(id, declarators, &mut t)?;
            }
            // `#sec-variable-statement-runtime-semantics-evaluation`.
            Stmt::Var(_, declarators) => self.declarators(id, declarators, &mut t)?,
            // `#sec-empty-statement-runtime-semantics-evaluation`,
            // `#sec-debugger-statement-runtime-semantics-evaluation` (no debugging facility), and a
            // function declaration, which FunctionDeclarationInstantiation,
            // GlobalDeclarationInstantiation, EvalDeclarationInstantiation or, in a block, the
            // entry of the block ([`Compiler::block_functions`]) instantiates
            // (`#sec-function-definitions-runtime-semantics-evaluation`).
            Stmt::Empty | Stmt::Debugger | Stmt::FunctionDeclaration(_) => {}
            // `#sec-expression-statement-runtime-semantics-evaluation`.
            Stmt::Expression(e) => {
                t.push(Task::Expr(*e))?;
                t.push(Task::Emit(self.statement_value(), span))?;
            }
            // `#sec-if-statement-runtime-semantics-evaluation`.
            Stmt::If {
                test,
                consequent,
                alternate,
            } => {
                self.reset_completion(span, &mut t)?;
                let otherwise = self.label();
                t.push(Task::Expr(*test))?;
                t.push(Task::Emit(Instruction::JumpIfFalse(otherwise), span))?;
                t.push(Task::Stmt(*consequent))?;
                match alternate {
                    Some(alternate) => {
                        let end = self.label();
                        t.push(Task::Goto(end, span))?;
                        t.push(Task::Bind(otherwise))?;
                        t.push(Task::Stmt(*alternate))?;
                        t.push(Task::Bind(end))?;
                    }
                    None => t.push(Task::Bind(otherwise))?,
                }
            }
            // `#sec-do-while-statement-runtime-semantics-labelledevaluation`.
            Stmt::DoWhile { body, test } => {
                self.reset_completion(span, &mut t)?;
                let (top, next, exit) = (self.label(), self.label(), self.label());
                t.push(Task::Bind(top))?;
                t.push(self.enter_loop(exit, next, self.items, std::mem::take(&mut labels)))?;
                t.push(Task::Stmt(*body))?;
                t.push(Task::PopControl(None))?;
                t.push(Task::Bind(next))?;
                t.push(Task::Expr(*test))?;
                t.push(Task::Emit(Instruction::JumpIfFalse(exit), span))?;
                t.push(Task::Goto(top, span))?;
                t.push(Task::Bind(exit))?;
            }
            // `#sec-while-statement-runtime-semantics-labelledevaluation`.
            Stmt::While { test, body } => {
                self.reset_completion(span, &mut t)?;
                let (top, exit) = (self.label(), self.label());
                t.push(Task::Bind(top))?;
                t.push(Task::Expr(*test))?;
                t.push(Task::Emit(Instruction::JumpIfFalse(exit), span))?;
                t.push(self.enter_loop(exit, top, self.items, std::mem::take(&mut labels)))?;
                t.push(Task::Stmt(*body))?;
                t.push(Task::PopControl(None))?;
                t.push(Task::Goto(top, span))?;
                t.push(Task::Bind(exit))?;
            }
            // `#sec-for-statement-runtime-semantics-labelledevaluation`, ForBodyEvaluation; a head
            // with `let` or `const` is a scope around the loop
            // (`#sec-runtime-semantics-forloopevaluation`), whose record a `let` head copies
            // for each iteration (`#sec-createperiterationenvironment`).
            Stmt::For {
                init,
                test,
                update,
                body,
            } => {
                self.reset_completion(span, &mut t)?;
                let mut record = false;
                let mut copy = false;
                match init {
                    Some(ForInit::Var(kind, declarators)) if kind.is_lexical() => {
                        record = self.enter_scope(ScopeKey::Head(id), span, &mut t)?;
                        copy = record && *kind == DeclKind::Let;
                        self.lexical_declarators(id, declarators, &mut t)?;
                    }
                    Some(ForInit::Var(_, declarators)) => {
                        self.declarators(id, declarators, &mut t)?;
                    }
                    Some(ForInit::Expression(e)) => {
                        t.push(Task::Expr(*e))?;
                        t.push(Task::Emit(Instruction::Pop, span))?;
                    }
                    None => {}
                }
                if copy {
                    t.push(Task::Emit(Instruction::CopyEnv, span))?;
                }
                let (top, next, exit) = (self.label(), self.label(), self.label());
                t.push(Task::Bind(top))?;
                if let Some(test) = test {
                    t.push(Task::Expr(*test))?;
                    t.push(Task::Emit(Instruction::JumpIfFalse(exit), span))?;
                }
                t.push(Task::PushControl(Control::Loop {
                    exit,
                    next,
                    items: self.items,
                    envs: self.envs.saturating_add(u32::from(record)),
                    labels: std::mem::take(&mut labels),
                }))?;
                t.push(Task::Stmt(*body))?;
                t.push(Task::PopControl(None))?;
                t.push(Task::Bind(next))?;
                if copy {
                    t.push(Task::Emit(Instruction::CopyEnv, span))?;
                }
                if let Some(update) = update {
                    t.push(Task::Expr(*update))?;
                    t.push(Task::Emit(Instruction::Pop, span))?;
                }
                t.push(Task::Goto(top, span))?;
                t.push(Task::Bind(exit))?;
                leave_scope(record, span, &mut t)?;
            }
            // `#sec-for-in-and-for-of-statements-runtime-semantics-labelledevaluation`:
            // ForIn/OfHeadEvaluation, then ForIn/OfBodyEvaluation
            // (`#sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset`),
            // the enumerator on the operand stack for the whole loop; for-of, the Iterator Record.
            Stmt::ForIn {
                target,
                object,
                body,
                iteration: IterationKind::Iterate,
            } => {
                self.for_of(id, span, target, *object, *body, labels, &mut t)?;
            }
            Stmt::ForIn {
                target,
                object,
                body,
                iteration: IterationKind::AsyncIterate,
            } => {
                self.for_await(id, span, target, *object, *body, labels, &mut t)?;
            }
            Stmt::ForIn {
                target: ForInTarget::Var(kind, target),
                object,
                body,
                ..
            } if kind.is_lexical() => {
                self.lexical_for_in(id, span, target, *object, *body, labels, &mut t)?;
            }
            Stmt::ForIn {
                target,
                object,
                body,
                ..
            } => {
                self.reset_completion(span, &mut t)?;
                let (top, exit) = (self.label(), self.label());
                t.push(Task::Expr(*object))?;
                t.push(Task::Emit(Instruction::ForInStart, span))?;
                t.push(Task::ItemsUp)?;
                t.push(Task::Bind(top))?;
                t.push(Task::Emit(Instruction::ForInNext(exit), span))?;
                self.put_to_head(id, span, target, &mut t)?;
                t.push(self.enter_loop(
                    exit,
                    top,
                    self.items.saturating_add(1),
                    std::mem::take(&mut labels),
                ))?;
                t.push(Task::Stmt(*body))?;
                t.push(Task::PopControl(None))?;
                t.push(Task::Goto(top, span))?;
                t.push(Task::Bind(exit))?;
                t.push(Task::Emit(Instruction::Pop, span))?;
                t.push(Task::ItemsDown)?;
            }
            // `#sec-continue-statement-runtime-semantics-evaluation`.
            Stmt::Continue(label) => {
                let label = match label {
                    Some(l) => Some(self.m.string(&l.name)?),
                    None => None,
                };
                t.push(Task::Jump(JumpKind::Continue, label, span))?;
            }
            // `#sec-break-statement-runtime-semantics-evaluation`.
            Stmt::Break(label) => {
                let label = match label {
                    Some(l) => Some(self.m.string(&l.name)?),
                    None => None,
                };
                t.push(Task::Jump(JumpKind::Break, label, span))?;
            }
            // `#sec-return-statement-runtime-semantics-evaluation`.
            Stmt::Return(e) => {
                match e {
                    Some(e) => t.push(Task::Expr(*e))?,
                    None => t.push(Task::Emit(Instruction::Undefined, span))?,
                }
                t.push(Task::Return(span))?;
            }
            // `#sec-switch-statement-runtime-semantics-evaluation` and CaseBlockEvaluation
            // (`#sec-runtime-semantics-caseblockevaluation`): the tests of the clauses before
            // and after the default clause, in source order, then the default clause; from the
            // clause chosen the statement lists run on in source order.
            Stmt::Switch {
                discriminant,
                cases,
            } => {
                self.reset_completion(span, &mut t)?;
                let exit = self.label();
                let mut labels = List::new(self.m);
                labels.extend(cases.iter().map(|_| self.label()))?;
                t.push(Task::Expr(*discriminant))?;
                let record = self.enter_scope(ScopeKey::Case(id), span, &mut t)?;
                for case in cases {
                    self.block_functions(&case.body, &mut t)?;
                }
                let mut default = None;
                for (case, label) in cases.iter().zip(labels.as_slice()) {
                    match case.test {
                        Some(test) => {
                            t.push(Task::Expr(test))?;
                            t.push(Task::Emit(Instruction::CaseJump(*label), case.span))?;
                        }
                        None => default = Some(*label),
                    }
                }
                t.push(Task::Emit(Instruction::Pop, span))?;
                t.push(Task::Goto(default.unwrap_or(exit), span))?;
                t.push(Task::PushControl(Control::Switch {
                    exit,
                    items: self.items,
                    envs: self.envs.saturating_add(u32::from(record)),
                }))?;
                for (case, label) in cases.iter().zip(labels.as_slice()) {
                    t.push(Task::Bind(*label))?;
                    t.extend(case.body.iter().map(|s| Task::Stmt(*s)))?;
                }
                t.push(Task::PopControl(None))?;
                t.push(Task::Bind(exit))?;
                leave_scope(record, span, &mut t)?;
            }
            // `#sec-throw-statement-runtime-semantics-evaluation`.
            Stmt::Throw(e) => {
                t.push(Task::Expr(*e))?;
                t.push(Task::Emit(Instruction::Throw, span))?;
            }
            // `#sec-with-statement-runtime-semantics-evaluation`: the object environment is an
            // environment of the frame, so every way out of the body pops it (`PopEnv` on a break,
            // continue or return through it, a handler's `envs` on a throw); the completion value
            // is UpdateEmpty(C, undefined).
            Stmt::With { object, body } => {
                self.reset_completion(span, &mut t)?;
                t.push(Task::Expr(*object))?;
                t.push(Task::Emit(Instruction::PushWith, span))?;
                t.push(Task::EnvsUp)?;
                t.push(Task::Stmt(*body))?;
                t.push(Task::Emit(Instruction::PopEnv, span))?;
                t.push(Task::EnvsDown)?;
            }
            Stmt::Try {
                block,
                handler,
                finalizer,
            } => self.try_statement(
                id,
                span,
                block,
                handler.is_some(),
                finalizer.as_deref(),
                &mut t,
            )?,
        }
        Ok(t)
    }

    /// What an expression statement does with its value.
    fn statement_value(&self) -> Instruction {
        if self.script {
            Instruction::StoreCompletion
        } else {
            Instruction::Pop
        }
    }

    /// In script code: the completion value register becomes `undefined`.
    fn reset_completion(&self, span: Span, t: &mut Tasks<'_, '_>) -> Result<(), CompileError> {
        if self.script {
            t.push(Task::Emit(Instruction::Undefined, span))?;
            t.push(Task::Emit(Instruction::StoreCompletion, span))?;
        }
        Ok(())
    }

    fn enter_loop(&self, exit: u32, next: u32, items: u32, labels: Vec<String>) -> Task {
        Task::PushControl(Control::Loop {
            exit,
            next,
            items,
            envs: self.envs,
            labels,
        })
    }

    /// The declarators of a `var` statement or `for` head: each with an initializer is
    /// ResolveBinding, the value, PutValue (`#sec-variable-statement-runtime-semantics-evaluation`).
    fn declarators(
        &mut self,
        id: StmtId,
        declarators: &[VarDeclarator],
        t: &mut Tasks<'_, '_>,
    ) -> Result<(), CompileError> {
        for (i, d) in declarators.iter().enumerate() {
            let Some(init) = d.init else { continue };
            // A pattern: the value, destructured (`#sec-runtime-semantics-bindinginitialization`).
            if let Some(pattern) = d.target.pattern() {
                t.push(Task::Expr(init))?;
                t.push(Task::Expr(pattern))?;
                t.push(Task::Emit(Instruction::Pop, d.target.span()))?;
                continue;
            }
            let n = u32::try_from(i).map_err(|_| crate::CompileError::TooMany("declarators"))?;
            let target = Target::Binding(self.binding(Site::VarDecl(id, n))?);
            let span = d.target.span();
            t.append(self.reference(&target, false, span)?)?;
            t.push(Task::Expr(init))?;
            t.push(Task::Emit(self.put(&target)?, span))?;
            t.push(Task::Emit(Instruction::Pop, span))?;
        }
        Ok(())
    }

    /// The entry of the lexical scope `key`, when the resolver gave it one: the record of its
    /// bindings that live in one, then its checked local slots uninitialized
    /// (`#sec-blockdeclarationinstantiation`). Whether it made a record, which the exit of the
    /// scope pops ([`leave_scope`]).
    pub(crate) fn enter_scope(
        &mut self,
        key: ScopeKey,
        span: Span,
        t: &mut Tasks<'_, '_>,
    ) -> Result<bool, CompileError> {
        let Some(scope) = self.resolution.lexical_scopes.get(&key) else {
            return Ok(false);
        };
        let record = !scope.env.is_empty();
        if record {
            let index = self.lexical_table(&scope.env)?;
            t.push(Task::Emit(Instruction::PushLexicalEnv(index), span))?;
            t.push(Task::EnvsUp)?;
        }
        for &n in &scope.uninitialize {
            t.push(Task::Emit(Instruction::Uninitialize(n), span))?;
        }
        Ok(record)
    }

    /// BlockDeclarationInstantiation, step 3.d (`#sec-blockdeclarationinstantiation`), after
    /// [`Compiler::enter_scope`]: each function the statement list `list` declares, in source
    /// order, made by the closure instruction in the scope just entered, its record (if any) the
    /// function's [[Environment]], and stored in its binding (InitializeBinding), before the
    /// scope's first statement (decision 3, "Function declarations in a block", item 1). The
    /// declaration itself compiles to nothing.
    fn block_functions(
        &mut self,
        list: &[StmtId],
        t: &mut Tasks<'_, '_>,
    ) -> Result<(), CompileError> {
        let ast = self.ast;
        for &s in list {
            let Some(Stmt::FunctionDeclaration(f)) = ast.stmt(s).map(|n| &n.stmt) else {
                continue;
            };
            let span = ast
                .function(*f)
                .map(|x| x.span)
                .ok_or_else(|| malformed("a function index"))?;
            let store = match self.binding(Site::Declaration(*f))? {
                Binding::Local(n) => Instruction::SetLocal(n),
                Binding::Env { hops, slot } => Instruction::SetEnv { hops, slot },
                _ => return Err(malformed("a block function's binding")),
            };
            t.push(Task::Emit(Instruction::Closure(f.0), span))?;
            t.push(Task::Emit(store, span))?;
            t.push(Task::Emit(Instruction::Pop, span))?;
        }
        Ok(())
    }

    /// The entry of the scope of the top level of the body of function `id`, before its
    /// function declarations are instantiated (`#sec-functiondeclarationinstantiation`, steps
    /// 30 to 36): emitted at once, at the start of the function's code.
    pub(crate) fn enter_function_body(
        &mut self,
        id: FunctionId,
        span: Span,
    ) -> Result<(), CompileError> {
        let mut t = Tasks::new(self.m);
        if self.enter_scope(ScopeKey::Body(id), span, &mut t)? {
            self.envs = self.envs.saturating_add(1);
        }
        for task in t.drain() {
            if let Task::Emit(instruction, span) = task {
                self.emit(instruction, span)?;
            }
        }
        Ok(())
    }

    /// A `scopes` entry naming the slots `names` of a record of `let` and `const` bindings.
    fn lexical_table(&mut self, names: &[(String, bool)]) -> Result<u32, CompileError> {
        let table = slot_names(self.constants, names, self.m)?;
        let index =
            u32::try_from(self.scopes.len()).map_err(|_| CompileError::TooMany("environments"))?;
        self.m.push(&mut self.scopes, table)?;
        Ok(index)
    }

    /// The declarators of a `let` or `const` declaration, or of a `for` head that is one: each
    /// value, `undefined` without an initializer, then InitializeBinding of its binding: the
    /// store into its slot, or into the global environment's [[DeclarativeRecord]] for a name
    /// of the script's top level (`#sec-let-and-const-declarations-runtime-semantics-evaluation`).
    fn lexical_declarators(
        &mut self,
        id: StmtId,
        declarators: &[VarDeclarator],
        t: &mut Tasks<'_, '_>,
    ) -> Result<(), CompileError> {
        for (i, d) in declarators.iter().enumerate() {
            let n = u32::try_from(i).map_err(|_| CompileError::TooMany("declarators"))?;
            let span = d.target.span();
            // A pattern, which always has an initializer: the value, destructured; the pattern
            // initializes each name it binds.
            if let Some(pattern) = d.target.pattern() {
                let init = d
                    .init
                    .ok_or_else(|| malformed("a pattern declaration without an initializer"))?;
                t.push(Task::Expr(init))?;
                t.push(Task::Expr(pattern))?;
                t.push(Task::Emit(Instruction::Pop, span))?;
                continue;
            }
            let store = match self.binding(Site::VarDecl(id, n))? {
                Binding::Local(n) => Instruction::SetLocal(n),
                Binding::Env { hops, slot } => Instruction::SetEnv { hops, slot },
                Binding::Global(name) | Binding::Dynamic(name) => {
                    Instruction::InitGlobal(self.string(&name)?)
                }
                _ => return Err(malformed("a lexical declaration's binding")),
            };
            match d.init {
                Some(init) => t.push(Task::Expr(init))?,
                None => t.push(Task::Emit(Instruction::Undefined, span))?,
            }
            t.push(Task::Emit(store, span))?;
            t.push(Task::Emit(Instruction::Pop, span))?;
        }
        Ok(())
    }

    /// A for-in with `let` or `const` (`#sec-runtime-semantics-forinofheadevaluation`, step 2,
    /// and `#sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset`,
    /// step 6.i): the expression in the scope of the head's names, uninitialized; each iteration
    /// in a scope of its own, whose binding the key initializes, or whose names the pattern of
    /// the head initializes from the key. A `continue` leaves the iteration's scope as a `break`
    /// does.
    #[allow(clippy::too_many_arguments)]
    fn lexical_for_in(
        &mut self,
        id: StmtId,
        span: Span,
        target: &BindingTarget,
        object: ExprId,
        body: StmtId,
        labels: Vec<String>,
        t: &mut Tasks<'_, '_>,
    ) -> Result<(), CompileError> {
        self.reset_completion(span, t)?;
        let (top, exit) = (self.label(), self.label());
        let tdz = self.enter_scope(ScopeKey::Tdz(id), span, t)?;
        t.push(Task::Expr(object))?;
        leave_scope(tdz, span, t)?;
        t.push(Task::Emit(Instruction::ForInStart, span))?;
        t.push(Task::ItemsUp)?;
        t.push(Task::Bind(top))?;
        t.push(Task::Emit(Instruction::ForInNext(exit), span))?;
        let iteration = self.enter_scope(ScopeKey::Iteration(id), span, t)?;
        self.initialize_head(id, target, t)?;
        t.push(Task::PushControl(Control::Loop {
            exit,
            next: top,
            items: self.items.saturating_add(1),
            envs: self.envs,
            labels,
        }))?;
        t.push(Task::Stmt(body))?;
        t.push(Task::PopControl(None))?;
        leave_scope(iteration, span, t)?;
        t.push(Task::Goto(top, span))?;
        t.push(Task::Bind(exit))?;
        t.push(Task::Emit(Instruction::Pop, span))?;
        t.push(Task::ItemsDown)?;
        Ok(())
    }

    /// The value on top, a key or a value of the iterator, initializes the binding of the
    /// `let` or `const` head of the for-in or for-of `id`, in the iteration's scope, or the
    /// pattern of the head initializes the names from it; then it is dropped
    /// (`#sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset`,
    /// steps 6.h and 6.i).
    fn initialize_head(
        &mut self,
        id: StmtId,
        target: &BindingTarget,
        t: &mut Tasks<'_, '_>,
    ) -> Result<(), CompileError> {
        let name_span = target.span();
        match target.pattern() {
            Some(pattern) => t.push(Task::Expr(pattern))?,
            None => {
                let store = match self.binding(Site::ForInVar(id))? {
                    Binding::Local(n) => Instruction::SetLocal(n),
                    Binding::Env { hops, slot } => Instruction::SetEnv { hops, slot },
                    _ => return Err(malformed("a for-in binding")),
                };
                t.push(Task::Emit(store, name_span))?;
            }
        }
        t.push(Task::Emit(Instruction::Pop, name_span))
    }

    /// The value on top, a key or a value of the iterator, goes to the `var` or left-hand side
    /// head of the for-in or for-of `id`, and is dropped: each iteration evaluates the target's
    /// reference, then puts, or destructures the value with the pattern of the head
    /// (`#sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset`,
    /// steps 6.g and 6.h).
    fn put_to_head(
        &mut self,
        id: StmtId,
        span: Span,
        target: &ForInTarget,
        t: &mut Tasks<'_, '_>,
    ) -> Result<(), CompileError> {
        let pattern = match target {
            ForInTarget::Var(_, target) => target.pattern(),
            ForInTarget::Expression(e) => self
                .ast
                .expr(*e)
                .filter(|n| {
                    matches!(
                        n.expr,
                        Expr::ObjectPattern { .. } | Expr::ArrayPattern { .. }
                    )
                })
                .map(|_| *e),
        };
        if let Some(pattern) = pattern {
            let pspan = self.ast.expr(pattern).map(|n| n.span).unwrap_or(span);
            t.push(Task::Expr(pattern))?;
            return t.push(Task::Emit(Instruction::Pop, pspan));
        }
        let (target, target_span) = match target {
            ForInTarget::Var(_, name) => (
                Target::Binding(self.binding(Site::ForInVar(id))?),
                name.span(),
            ),
            ForInTarget::Expression(e) => {
                let span = self.ast.expr(*e).map(|n| n.span).unwrap_or(span);
                (self.target(*e)?, span)
            }
        };
        let r = target.reference_items();
        t.append(self.reference(&target, false, target_span)?)?;
        for _ in 0..r {
            t.push(Task::Emit(Instruction::Bury(r), target_span))?;
        }
        t.push(Task::Emit(self.put(&target)?, target_span))?;
        t.push(Task::Emit(Instruction::Pop, target_span))
    }

    /// A for-of (`#sec-runtime-semantics-forinofheadevaluation` and
    /// `#sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset`
    /// for `iterate`): the expression, in the scope of the head's `let` or `const` names,
    /// uninitialized; the Iterator Record on the operand stack for the whole loop; each step
    /// puts its value to the head, in a scope of its own for `let` and `const`. Inside the loop's
    /// control entry, a [`Control::Finally`] that `closes`: a throw from the step to the end of
    /// the body, a `return`, and a `break` or `continue` that leaves the loop enter
    /// [`Instruction::IteratorClose`] with their completion record, then go on as after a
    /// `finally` block (ForIn/OfBodyEvaluation, steps 6.i.iii, 6.k and 6.l.iii). The step jumps
    /// past it when the iterator is done, and a `continue` of the loop itself goes past it.
    #[allow(clippy::too_many_arguments)]
    fn for_of(
        &mut self,
        id: StmtId,
        span: Span,
        target: &ForInTarget,
        object: ExprId,
        body: StmtId,
        labels: Vec<String>,
        t: &mut Tasks<'_, '_>,
    ) -> Result<(), CompileError> {
        let lexical = matches!(target, ForInTarget::Var(kind, _) if kind.is_lexical());
        self.reset_completion(span, t)?;
        let (items, envs) = (self.items.saturating_add(1), self.envs);
        let (top, end, entry, exit) = (self.label(), self.label(), self.label(), self.label());
        let key = self.finally_key();
        let tdz = lexical && self.enter_scope(ScopeKey::Tdz(id), span, t)?;
        t.push(Task::Expr(object))?;
        leave_scope(tdz, span, t)?;
        let object_span = self.ast.expr(object).map(|n| n.span).unwrap_or(span);
        t.push(Task::Emit(Instruction::GetIterator, object_span))?;
        t.push(Task::ItemsUp)?;
        t.push(Task::PushControl(Control::Loop {
            exit,
            next: top,
            items,
            envs,
            labels,
        }))?;
        t.push(Task::PushControl(Control::Finally {
            entry,
            items,
            envs,
            returns: false,
            jumps: Vec::new(),
            closes: true,
        }))?;
        t.push(Task::Bind(top))?;
        t.push(Task::Emit(Instruction::IteratorStep(exit), span))?;
        let iteration = match target {
            ForInTarget::Var(kind, target) if kind.is_lexical() => {
                let iteration = self.enter_scope(ScopeKey::Iteration(id), span, t)?;
                self.initialize_head(id, target, t)?;
                iteration
            }
            _ => {
                self.put_to_head(id, span, target, t)?;
                false
            }
        };
        t.push(Task::Stmt(body))?;
        leave_scope(iteration, span, t)?;
        t.push(Task::PopControl(Some(key)))?;
        t.push(Task::Goto(top, span))?;
        t.push(Task::Bind(end))?;
        t.push(Task::Bind(entry))?;
        t.push(Task::ItemsUp)?;
        t.push(Task::ItemsUp)?;
        t.push(Task::Emit(Instruction::IteratorClose, span))?;
        t.push(Task::FinallyDispatch(key, span))?;
        t.push(Task::PopControl(None))?;
        t.push(Task::Bind(exit))?;
        t.push(Task::Emit(Instruction::Pop, span))?;
        t.push(Task::ItemsDown)?;
        t.push(Task::Handler {
            kind: HandlerKind::Finally,
            start: top,
            end,
            target: entry,
            items,
            envs,
        })
    }

    /// A for-await-of in an `async` function (`#sec-runtime-semantics-forinofheadevaluation`
    /// and `#sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset`
    /// for `async-iterate` and iteratorKind async): as [`Compiler::for_of`], with
    /// [`Instruction::GetAsyncIterator`] for the head, and for each step the call of `next`
    /// ([`Instruction::AsyncIteratorNext`]), the `await` of its result as an `await` expression
    /// awaits (a rejection throws there), and the check of the awaited result
    /// ([`Instruction::AsyncIteratorStep`]); a throw of the three leaves the loop with no close,
    /// so the region of the [`Control::Finally`] that `closes` starts after them. Its block is
    /// AsyncIteratorClose (`#sec-asynciteratorclose`): [`Instruction::AsyncIteratorClose`], the
    /// `await` of the result of `return`, and [`Instruction::AsyncIteratorClosed`]; a throw of
    /// that `await` itself (of PromiseResolve) enters a [`HandlerKind::Finally`] handler whose
    /// target is [`Instruction::AsyncIteratorClosed`], which receives it as a rejection.
    #[allow(clippy::too_many_arguments)]
    fn for_await(
        &mut self,
        id: StmtId,
        span: Span,
        target: &ForInTarget,
        object: ExprId,
        body: StmtId,
        labels: Vec<String>,
        t: &mut Tasks<'_, '_>,
    ) -> Result<(), CompileError> {
        let slot = self
            .asynchronous
            .ok_or_else(|| malformed("a for await outside an async function"))?;
        let lexical = matches!(target, ForInTarget::Var(kind, _) if kind.is_lexical());
        self.reset_completion(span, t)?;
        let (items, envs) = (self.items.saturating_add(1), self.envs);
        let (top, normal, start, end, entry, exit) = (
            self.label(),
            self.label(),
            self.label(),
            self.label(),
            self.label(),
            self.label(),
        );
        let (close_await, awaited, closed) = (self.label(), self.label(), self.label());
        let key = self.finally_key();
        let tdz = lexical && self.enter_scope(ScopeKey::Tdz(id), span, t)?;
        t.push(Task::Expr(object))?;
        leave_scope(tdz, span, t)?;
        let object_span = self.ast.expr(object).map(|n| n.span).unwrap_or(span);
        t.push(Task::Emit(Instruction::GetAsyncIterator, object_span))?;
        t.push(Task::ItemsUp)?;
        t.push(Task::PushControl(Control::Loop {
            exit,
            next: top,
            items,
            envs,
            labels,
        }))?;
        t.push(Task::PushControl(Control::Finally {
            entry,
            items,
            envs,
            returns: false,
            jumps: Vec::new(),
            closes: true,
        }))?;
        t.push(Task::Bind(top))?;
        t.push(Task::Emit(Instruction::AsyncIteratorNext, span))?;
        t.push(Task::Emit(Instruction::Await(slot), span))?;
        t.push(Task::Emit(
            Instruction::JumpIfTag {
                tag: TAG_NORMAL,
                target: normal,
            },
            span,
        ))?;
        t.push(Task::Emit(Instruction::EndFinally, span))?;
        t.push(Task::Bind(normal))?;
        t.push(Task::Emit(Instruction::AsyncIteratorStep(exit), span))?;
        t.push(Task::Bind(start))?;
        let iteration = match target {
            ForInTarget::Var(kind, target) if kind.is_lexical() => {
                let iteration = self.enter_scope(ScopeKey::Iteration(id), span, t)?;
                self.initialize_head(id, target, t)?;
                iteration
            }
            _ => {
                self.put_to_head(id, span, target, t)?;
                false
            }
        };
        t.push(Task::Stmt(body))?;
        leave_scope(iteration, span, t)?;
        t.push(Task::PopControl(Some(key)))?;
        t.push(Task::Goto(top, span))?;
        t.push(Task::Bind(end))?;
        t.push(Task::Bind(entry))?;
        t.push(Task::ItemsUp)?;
        t.push(Task::ItemsUp)?;
        t.push(Task::Emit(Instruction::AsyncIteratorClose(closed), span))?;
        t.push(Task::Bind(close_await))?;
        t.push(Task::Emit(Instruction::Await(slot), span))?;
        t.push(Task::Bind(awaited))?;
        t.push(Task::Emit(Instruction::AsyncIteratorClosed, span))?;
        t.push(Task::Bind(closed))?;
        t.push(Task::FinallyDispatch(key, span))?;
        t.push(Task::PopControl(None))?;
        t.push(Task::Bind(exit))?;
        t.push(Task::Emit(Instruction::Pop, span))?;
        t.push(Task::ItemsDown)?;
        t.push(Task::Handler {
            kind: HandlerKind::Finally,
            start,
            end,
            target: entry,
            items,
            envs,
        })?;
        // The `await` of the result of `return`, the one instruction from `close_await` to
        // `awaited`.
        t.push(Task::Handler {
            kind: HandlerKind::Finally,
            start: close_await,
            end: awaited,
            target: awaited,
            items: items.saturating_add(2),
            envs,
        })?;
        Ok(())
    }

    /// `#sec-try-statement-runtime-semantics-evaluation`, with CatchClauseEvaluation
    /// (`#sec-runtime-semantics-catchclauseevaluation`). The catch handler covers the try block;
    /// the finally handler covers the try block and the catch block, and the `finally` block is
    /// entered with a completion record by every way out of them.
    fn try_statement(
        &mut self,
        id: StmtId,
        span: Span,
        block: &[StmtId],
        catch: bool,
        finalizer: Option<&[StmtId]>,
        t: &mut Tasks<'_, '_>,
    ) -> Result<(), CompileError> {
        self.reset_completion(span, t)?;
        let (items, envs) = (self.items, self.envs);
        let start = self.label();
        let block_end = self.label();
        let normal = self.label();
        let entry = self.label();
        let key = self.finally_key();
        t.push(Task::Bind(start))?;
        if finalizer.is_some() {
            t.push(Task::PushControl(Control::Finally {
                entry,
                items,
                envs,
                returns: false,
                jumps: Vec::new(),
                closes: false,
            }))?;
        }
        let record = self.enter_scope(ScopeKey::Try(id), span, t)?;
        self.block_functions(block, t)?;
        t.extend(block.iter().map(|s| Task::Stmt(*s)))?;
        leave_scope(record, span, t)?;
        let catch_end = if catch {
            let Some(Stmt::Try {
                handler: Some(h), ..
            }) = self.ast.stmt(id).map(|n| &n.stmt)
            else {
                return Err(malformed("a catch clause"));
            };
            let catch_end = self.label();
            t.push(Task::Goto(normal, span))?;
            t.push(Task::Bind(block_end))?;
            // The exception is on top: it initializes the parameter, in an environment of its
            // own when a function inside the clause uses it. A `catch` without a parameter
            // drops it and makes no environment (`Catch : catch Block`).
            // A pattern: the scope of its names, uninitialized, then the exception destructured
            // into them (`#sec-runtime-semantics-catchclauseevaluation`, steps 2 to 5).
            let mut names = false;
            let env = match &h.param {
                Some(BindingTarget::Pattern(p)) => {
                    names = self.enter_scope(ScopeKey::CatchParameter(id), p.span, t)?;
                    t.push(Task::Expr(p.pattern))?;
                    t.push(Task::Emit(Instruction::Pop, p.span))?;
                    false
                }
                Some(BindingTarget::Identifier(param)) => {
                    let pspan = param.span;
                    let env = match self.binding(Site::CatchParam(id))? {
                        Binding::Local(n) => {
                            t.push(Task::Emit(Instruction::SetLocal(n), pspan))?;
                            false
                        }
                        Binding::Env { slot, .. } => {
                            // Named when it encloses a `with` body (`resolve.rs`).
                            let push = if self.resolution.named_catches.contains(&id) {
                                let name = SlotName::plain(self.string(&param.name)?)
                                    .ok_or(crate::CompileError::TooMany("strings"))?;
                                let index = u32::try_from(self.scopes.len())
                                    .map_err(|_| crate::CompileError::TooMany("environments"))?;
                                let mut table = self.m.with_capacity(1)?;
                                table.push(name);
                                self.m.push(&mut self.scopes, table)?;
                                Instruction::PushNamedEnv(index)
                            } else {
                                Instruction::PushEnv(1)
                            };
                            t.push(Task::Emit(push, pspan))?;
                            t.push(Task::Emit(Instruction::SetEnv { hops: 0, slot }, pspan))?;
                            t.push(Task::EnvsUp)?;
                            true
                        }
                        _ => return Err(malformed("a catch parameter")),
                    };
                    t.push(Task::Emit(Instruction::Pop, pspan))?;
                    env
                }
                None => {
                    t.push(Task::Emit(Instruction::Pop, span))?;
                    false
                }
            };
            self.reset_completion(span, t)?;
            let record = self.enter_scope(ScopeKey::Catch(id), span, t)?;
            self.block_functions(&h.body, t)?;
            t.extend(h.body.iter().map(|s| Task::Stmt(*s)))?;
            leave_scope(record, span, t)?;
            leave_scope(names, span, t)?;
            if env {
                t.push(Task::Emit(Instruction::PopEnv, span))?;
                t.push(Task::EnvsDown)?;
            }
            t.push(Task::Bind(catch_end))?;
            catch_end
        } else {
            t.push(Task::Bind(block_end))?;
            block_end
        };
        match finalizer {
            Some(list) => {
                t.push(Task::PopControl(Some(key)))?;
                t.push(Task::Bind(normal))?;
                t.push(Task::Emit(Instruction::Undefined, span))?;
                t.push(Task::Emit(Instruction::Tag(TAG_NORMAL), span))?;
                t.push(Task::Bind(entry))?;
                t.push(Task::ItemsUp)?;
                t.push(Task::ItemsUp)?;
                if self.script {
                    t.push(Task::Emit(Instruction::LoadCompletion, span))?;
                    t.push(Task::ItemsUp)?;
                }
                // A break or continue out of the finally block leaves
                // UpdateEmpty(finallyResult, undefined): its own value or undefined, not the
                // value it kept.
                self.reset_completion(span, t)?;
                let record = self.enter_scope(ScopeKey::Finally(id), span, t)?;
                self.block_functions(list, t)?;
                t.extend(list.iter().map(|s| Task::Stmt(*s)))?;
                leave_scope(record, span, t)?;
                if self.script {
                    t.push(Task::Emit(Instruction::StoreCompletion, span))?;
                    t.push(Task::ItemsDown)?;
                }
                t.push(Task::FinallyDispatch(key, span))?;
            }
            None => t.push(Task::Bind(normal))?,
        }
        if catch {
            t.push(Task::Handler {
                kind: HandlerKind::Catch,
                start,
                end: block_end,
                target: block_end,
                items,
                envs,
            })?;
        }
        if finalizer.is_some() {
            t.push(Task::Handler {
                kind: HandlerKind::Finally,
                start,
                end: catch_end,
                target: entry,
                items,
                envs,
            })?;
        }
        Ok(())
    }
}
