//! Statements (clause 14, `#sec-ecmascript-language-statements-and-declarations`): block, `var`,
//! empty, expression, `if`, `do-while`, `while`, `for`, `for-in` with `var` or a left-hand side,
//! `continue`, `break`, `return`, `switch`, `throw`, `try` with `catch` and `finally`,
//! `debugger`, labelled statements with `break` and `continue` to a label, and `with` in non-strict
//! code (its body runs in an Object Environment Record; the names in it are looked up at run
//! time, `resolve.rs`).
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

use swell_syntax::ast::{ForInTarget, ForInit, Span, Stmt, StmtId, VarDeclarator};

use crate::bytecode::{HandlerKind, Instruction, TAG_NORMAL};
use crate::expression::Target;
use crate::resolve::{Binding, Site};
use crate::{CompileError, Compiler, Control, JumpKind, Task, malformed};

impl<'a> Compiler<'a> {
    /// The tasks of the statement `id`; a loop gets the label set `labels`.
    pub(crate) fn statement(
        &mut self,
        id: StmtId,
        mut labels: Vec<String>,
    ) -> Result<Vec<Task>, CompileError> {
        let node = self
            .ast
            .stmt(id)
            .ok_or_else(|| malformed("a statement index"))?;
        let span = node.span;
        let mut t = Vec::new();
        match &node.stmt {
            // `#sec-labelled-statements-runtime-semantics-labelledevaluation`: the labels of a run
            // of labelled statements go to the statement they label.
            Stmt::Labelled { .. } => {
                let mut item = id;
                while let Some(Stmt::Labelled { label, body }) =
                    self.ast.stmt(item).map(|n| &n.stmt)
                {
                    labels.push(label.name.clone());
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
                    t.push(Task::LabelledLoop(item, labels));
                } else {
                    let exit = self.label();
                    t.push(Task::PushControl(Control::Label {
                        labels,
                        exit,
                        items: self.items,
                        envs: self.envs,
                    }));
                    t.push(Task::Stmt(item));
                    t.push(Task::PopControl(None));
                    t.push(Task::Bind(exit));
                }
            }
            // `#sec-block-runtime-semantics-evaluation`.
            Stmt::Block(list) => t.extend(list.iter().map(|s| Task::Stmt(*s))),
            // `#sec-variable-statement-runtime-semantics-evaluation`.
            Stmt::Var(declarators) => self.declarators(id, declarators, &mut t)?,
            // `#sec-empty-statement-runtime-semantics-evaluation`,
            // `#sec-debugger-statement-runtime-semantics-evaluation` (no debugging facility), and a
            // function declaration, which FunctionDeclarationInstantiation or
            // GlobalDeclarationInstantiation instantiates
            // (`#sec-function-definitions-runtime-semantics-evaluation`).
            Stmt::Empty | Stmt::Debugger | Stmt::FunctionDeclaration(_) => {}
            // `#sec-expression-statement-runtime-semantics-evaluation`.
            Stmt::Expression(e) => {
                t.push(Task::Expr(*e));
                t.push(Task::Emit(self.statement_value(), span));
            }
            // `#sec-if-statement-runtime-semantics-evaluation`.
            Stmt::If {
                test,
                consequent,
                alternate,
            } => {
                self.reset_completion(span, &mut t);
                let otherwise = self.label();
                t.push(Task::Expr(*test));
                t.push(Task::Emit(Instruction::JumpIfFalse(otherwise), span));
                t.push(Task::Stmt(*consequent));
                match alternate {
                    Some(alternate) => {
                        let end = self.label();
                        t.push(Task::Goto(end, span));
                        t.push(Task::Bind(otherwise));
                        t.push(Task::Stmt(*alternate));
                        t.push(Task::Bind(end));
                    }
                    None => t.push(Task::Bind(otherwise)),
                }
            }
            // `#sec-do-while-statement-runtime-semantics-labelledevaluation`.
            Stmt::DoWhile { body, test } => {
                self.reset_completion(span, &mut t);
                let (top, next, exit) = (self.label(), self.label(), self.label());
                t.push(Task::Bind(top));
                t.push(self.enter_loop(exit, next, self.items, std::mem::take(&mut labels)));
                t.push(Task::Stmt(*body));
                t.push(Task::PopControl(None));
                t.push(Task::Bind(next));
                t.push(Task::Expr(*test));
                t.push(Task::Emit(Instruction::JumpIfFalse(exit), span));
                t.push(Task::Goto(top, span));
                t.push(Task::Bind(exit));
            }
            // `#sec-while-statement-runtime-semantics-labelledevaluation`.
            Stmt::While { test, body } => {
                self.reset_completion(span, &mut t);
                let (top, exit) = (self.label(), self.label());
                t.push(Task::Bind(top));
                t.push(Task::Expr(*test));
                t.push(Task::Emit(Instruction::JumpIfFalse(exit), span));
                t.push(self.enter_loop(exit, top, self.items, std::mem::take(&mut labels)));
                t.push(Task::Stmt(*body));
                t.push(Task::PopControl(None));
                t.push(Task::Goto(top, span));
                t.push(Task::Bind(exit));
            }
            // `#sec-for-statement-runtime-semantics-labelledevaluation`, ForBodyEvaluation.
            Stmt::For {
                init,
                test,
                update,
                body,
            } => {
                self.reset_completion(span, &mut t);
                match init {
                    Some(ForInit::Var(declarators)) => self.declarators(id, declarators, &mut t)?,
                    Some(ForInit::Expression(e)) => {
                        t.push(Task::Expr(*e));
                        t.push(Task::Emit(Instruction::Pop, span));
                    }
                    None => {}
                }
                let (top, next, exit) = (self.label(), self.label(), self.label());
                t.push(Task::Bind(top));
                if let Some(test) = test {
                    t.push(Task::Expr(*test));
                    t.push(Task::Emit(Instruction::JumpIfFalse(exit), span));
                }
                t.push(self.enter_loop(exit, next, self.items, std::mem::take(&mut labels)));
                t.push(Task::Stmt(*body));
                t.push(Task::PopControl(None));
                t.push(Task::Bind(next));
                if let Some(update) = update {
                    t.push(Task::Expr(*update));
                    t.push(Task::Emit(Instruction::Pop, span));
                }
                t.push(Task::Goto(top, span));
                t.push(Task::Bind(exit));
            }
            // `#sec-for-in-and-for-of-statements-runtime-semantics-labelledevaluation`:
            // ForIn/OfHeadEvaluation, then ForIn/OfBodyEvaluation
            // (`#sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset`),
            // the enumerator on the operand stack for the whole loop.
            Stmt::ForIn {
                target,
                object,
                body,
            } => {
                self.reset_completion(span, &mut t);
                let (top, exit) = (self.label(), self.label());
                t.push(Task::Expr(*object));
                t.push(Task::Emit(Instruction::ForInStart, span));
                t.push(Task::ItemsUp);
                t.push(Task::Bind(top));
                t.push(Task::Emit(Instruction::ForInNext(exit), span));
                // The key is on top; each iteration evaluates the target's reference, then puts.
                let (target, target_span) = match target {
                    ForInTarget::Var(name) => (
                        Target::Binding(self.binding(Site::ForInVar(id))?),
                        name.span,
                    ),
                    ForInTarget::Expression(e) => {
                        let span = self.ast.expr(*e).map(|n| n.span).unwrap_or(span);
                        (self.target(*e)?, span)
                    }
                };
                let r = target.reference_items();
                t.extend(self.reference(&target, false, target_span)?);
                for _ in 0..r {
                    t.push(Task::Emit(Instruction::Bury(r), target_span));
                }
                t.push(Task::Emit(self.put(&target)?, target_span));
                t.push(Task::Emit(Instruction::Pop, target_span));
                t.push(self.enter_loop(
                    exit,
                    top,
                    self.items.saturating_add(1),
                    std::mem::take(&mut labels),
                ));
                t.push(Task::Stmt(*body));
                t.push(Task::PopControl(None));
                t.push(Task::Goto(top, span));
                t.push(Task::Bind(exit));
                t.push(Task::Emit(Instruction::Pop, span));
                t.push(Task::ItemsDown);
            }
            // `#sec-continue-statement-runtime-semantics-evaluation`.
            Stmt::Continue(label) => t.push(Task::Jump(
                JumpKind::Continue,
                label.as_ref().map(|l| l.name.clone()),
                span,
            )),
            // `#sec-break-statement-runtime-semantics-evaluation`.
            Stmt::Break(label) => t.push(Task::Jump(
                JumpKind::Break,
                label.as_ref().map(|l| l.name.clone()),
                span,
            )),
            // `#sec-return-statement-runtime-semantics-evaluation`.
            Stmt::Return(e) => {
                match e {
                    Some(e) => t.push(Task::Expr(*e)),
                    None => t.push(Task::Emit(Instruction::Undefined, span)),
                }
                t.push(Task::Return(span));
            }
            // `#sec-switch-statement-runtime-semantics-evaluation` and CaseBlockEvaluation
            // (`#sec-runtime-semantics-caseblockevaluation`): the tests of the clauses before
            // and after the default clause, in source order, then the default clause; from the
            // clause chosen the statement lists run on in source order.
            Stmt::Switch {
                discriminant,
                cases,
            } => {
                self.reset_completion(span, &mut t);
                let exit = self.label();
                let labels: Vec<_> = cases.iter().map(|_| self.label()).collect();
                t.push(Task::Expr(*discriminant));
                let mut default = None;
                for (case, label) in cases.iter().zip(&labels) {
                    match case.test {
                        Some(test) => {
                            t.push(Task::Expr(test));
                            t.push(Task::Emit(Instruction::CaseJump(*label), case.span));
                        }
                        None => default = Some(*label),
                    }
                }
                t.push(Task::Emit(Instruction::Pop, span));
                t.push(Task::Goto(default.unwrap_or(exit), span));
                t.push(Task::PushControl(Control::Switch {
                    exit,
                    items: self.items,
                    envs: self.envs,
                }));
                for (case, label) in cases.iter().zip(&labels) {
                    t.push(Task::Bind(*label));
                    t.extend(case.body.iter().map(|s| Task::Stmt(*s)));
                }
                t.push(Task::PopControl(None));
                t.push(Task::Bind(exit));
            }
            // `#sec-throw-statement-runtime-semantics-evaluation`.
            Stmt::Throw(e) => {
                t.push(Task::Expr(*e));
                t.push(Task::Emit(Instruction::Throw, span));
            }
            // `#sec-with-statement-runtime-semantics-evaluation`: the object environment is an
            // environment of the frame, so every way out of the body pops it (`PopEnv` on a break,
            // continue or return through it, a handler's `envs` on a throw); the completion value
            // is UpdateEmpty(C, undefined).
            Stmt::With { object, body } => {
                self.reset_completion(span, &mut t);
                t.push(Task::Expr(*object));
                t.push(Task::Emit(Instruction::PushWith, span));
                t.push(Task::EnvsUp);
                t.push(Task::Stmt(*body));
                t.push(Task::Emit(Instruction::PopEnv, span));
                t.push(Task::EnvsDown);
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
    fn reset_completion(&self, span: Span, t: &mut Vec<Task>) {
        if self.script {
            t.push(Task::Emit(Instruction::Undefined, span));
            t.push(Task::Emit(Instruction::StoreCompletion, span));
        }
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
        t: &mut Vec<Task>,
    ) -> Result<(), CompileError> {
        for (i, d) in declarators.iter().enumerate() {
            let Some(init) = d.init else { continue };
            let n = u32::try_from(i).map_err(|_| crate::CompileError::TooMany("declarators"))?;
            let target = Target::Binding(self.binding(Site::VarDecl(id, n))?);
            let span = d.name.span;
            t.extend(self.reference(&target, false, span)?);
            t.push(Task::Expr(init));
            t.push(Task::Emit(self.put(&target)?, span));
            t.push(Task::Emit(Instruction::Pop, span));
        }
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
        t: &mut Vec<Task>,
    ) -> Result<(), CompileError> {
        self.reset_completion(span, t);
        let (items, envs) = (self.items, self.envs);
        let start = self.label();
        let block_end = self.label();
        let normal = self.label();
        let entry = self.label();
        let key = self.finally_key();
        t.push(Task::Bind(start));
        if finalizer.is_some() {
            t.push(Task::PushControl(Control::Finally {
                entry,
                items,
                envs,
                returns: false,
                jumps: Vec::new(),
            }));
        }
        t.extend(block.iter().map(|s| Task::Stmt(*s)));
        let catch_end = if catch {
            let Some(Stmt::Try {
                handler: Some(h), ..
            }) = self.ast.stmt(id).map(|n| &n.stmt)
            else {
                return Err(malformed("a catch clause"));
            };
            let catch_end = self.label();
            t.push(Task::Goto(normal, span));
            t.push(Task::Bind(block_end));
            // The exception is on top: it initializes the parameter, in an environment of its
            // own when a function inside the clause uses it. A `catch` without a parameter
            // drops it and makes no environment (`Catch : catch Block`).
            let env = match &h.param {
                Some(param) => {
                    let pspan = param.span;
                    let env = match self.binding(Site::CatchParam(id))? {
                        Binding::Local(n) => {
                            t.push(Task::Emit(Instruction::SetLocal(n), pspan));
                            false
                        }
                        Binding::Env { slot, .. } => {
                            // Named when it encloses a `with` body (`resolve.rs`).
                            let push = if self.resolution.named_catches.contains(&id) {
                                let name = self.string(&param.name)?;
                                let index = u32::try_from(self.scopes.len())
                                    .map_err(|_| crate::CompileError::TooMany("environments"))?;
                                self.scopes.push(vec![name]);
                                Instruction::PushNamedEnv(index)
                            } else {
                                Instruction::PushEnv(1)
                            };
                            t.push(Task::Emit(push, pspan));
                            t.push(Task::Emit(Instruction::SetEnv { hops: 0, slot }, pspan));
                            t.push(Task::EnvsUp);
                            true
                        }
                        _ => return Err(malformed("a catch parameter")),
                    };
                    t.push(Task::Emit(Instruction::Pop, pspan));
                    env
                }
                None => {
                    t.push(Task::Emit(Instruction::Pop, span));
                    false
                }
            };
            self.reset_completion(span, t);
            t.extend(h.body.iter().map(|s| Task::Stmt(*s)));
            if env {
                t.push(Task::Emit(Instruction::PopEnv, span));
                t.push(Task::EnvsDown);
            }
            t.push(Task::Bind(catch_end));
            catch_end
        } else {
            t.push(Task::Bind(block_end));
            block_end
        };
        match finalizer {
            Some(list) => {
                t.push(Task::PopControl(Some(key)));
                t.push(Task::Bind(normal));
                t.push(Task::Emit(Instruction::Undefined, span));
                t.push(Task::Emit(Instruction::Tag(TAG_NORMAL), span));
                t.push(Task::Bind(entry));
                t.push(Task::ItemsUp);
                t.push(Task::ItemsUp);
                if self.script {
                    t.push(Task::Emit(Instruction::LoadCompletion, span));
                    t.push(Task::ItemsUp);
                }
                // A break or continue out of the finally block leaves
                // UpdateEmpty(finallyResult, undefined): its own value or undefined, not the
                // value it kept.
                self.reset_completion(span, t);
                t.extend(list.iter().map(|s| Task::Stmt(*s)));
                if self.script {
                    t.push(Task::Emit(Instruction::StoreCompletion, span));
                    t.push(Task::ItemsDown);
                }
                t.push(Task::FinallyDispatch(key, span));
            }
            None => t.push(Task::Bind(normal)),
        }
        if catch {
            t.push(Task::Handler {
                kind: HandlerKind::Catch,
                start,
                end: block_end,
                target: block_end,
                items,
                envs,
            });
        }
        if finalizer.is_some() {
            t.push(Task::Handler {
                kind: HandlerKind::Finally,
                start,
                end: catch_end,
                target: entry,
                items,
                envs,
            });
        }
        Ok(())
    }
}
