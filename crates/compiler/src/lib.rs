#![forbid(unsafe_code)]
//! `swell-compiler`: the compile step of swell (`docs/design/engine.md`, decisions 2 and 3). It
//! turns a `Script` that `swell-syntax` parsed and analysed into the stack bytecode of
//! [`bytecode`], with no heap value: string and Number constants are kept as code units and
//! `f64`s, and the interpreter interns them.
//!
//! Slice 1 compiles the ES5 grammar the parser accepts: the statements of clause 14
//! (`statement.rs`), the expressions of clause 13 (`expression.rs`), and function declarations
//! and expressions of 15.2 (`function.rs`), with the runtime semantics of each clause in their
//! evaluation order. Slice 2 adds labelled statements, `??`, optional chains, the logical
//! assignments, a `catch` without a parameter, and plain methods and shorthand properties of
//! object literals, the `with` statement with the names its body looks up at run time, and the
//! arguments object of a function (`docs/design/engine.md`, decision 3, "Names"). A regular
//! expression literal compiles to the `SyntaxError` the design gives
//! it before slice 5 ([`swell_syntax::REGEXP_REFUSAL`]). Slots and environments are assigned by
//! `resolve.rs`; `disasm.rs` writes a [`Code`] as text.
//!
//! # Bounds
//!
//! The compiler walks only trees the parser accepted, whose depth the host's `parser_nesting`
//! bounds (decision 3, "Depth bounds"). It does not recurse on the Rust stack at all: every walk
//! keeps its pending work on a heap stack of its own, so a tree as deep as the parser admits
//! compiles on a small thread stack. Its own loops run over the nodes of the tree, so the
//! compile time is bounded by the size of the tree; nothing the compiler does runs for a
//! script, and no budget applies to it.

pub mod bytecode;
pub mod disasm;
mod expression;
mod function;
mod resolve;
mod statement;

use std::collections::BTreeMap;
use std::fmt;

use swell_syntax::Script;
use swell_syntax::ast::{Ast, ExprId, Span, StmtId};

use crate::bytecode::{Code, Handler, HandlerKind, Instruction, TAG_FIRST_JUMP, TAG_RETURN};
use crate::resolve::{Binding, Resolution, Site};

/// Why a script did not compile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompileError {
    /// A bound of the format: one function would need more than `u32::MAX` instructions,
    /// constants, slots or another counted thing, named here.
    TooMany(&'static str),
    /// The tree is not one that `swell_syntax::parse_script` returns: an index names no node, a
    /// literal's text is no literal, a `break` stands outside a loop. Never for a parsed script.
    Tree(String),
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileError::TooMany(what) => {
                write!(
                    f,
                    "more {what} in one function than the bytecode can number"
                )
            }
            CompileError::Tree(what) => write!(f, "not a tree the parser produces: {what}"),
        }
    }
}

impl std::error::Error for CompileError {}

/// Compiles a parsed script into its [`Code`]: the script's own instructions and a template for
/// every function in it.
pub fn compile_script(script: &Script) -> Result<Code, CompileError> {
    let resolution = resolve::resolve(script)?;
    let mut constants = Constants::default();
    let body = function::compile_script_body(script, &resolution, &mut constants)?;
    let mut functions = Vec::with_capacity(script.ast.functions().len());
    for i in 0..script.ast.functions().len() {
        let id = swell_syntax::ast::FunctionId(
            u32::try_from(i).map_err(|_| CompileError::TooMany("functions"))?,
        );
        functions.push(function::compile_function(
            &script.ast,
            id,
            &resolution,
            &mut constants,
        )?);
    }
    let mut var_names = Vec::new();
    let declared: Vec<&str> = script
        .scope
        .functions
        .iter()
        .filter_map(|f| script.ast.function(*f))
        .filter_map(|f| f.name.as_ref().map(|n| n.name.as_str()))
        .collect();
    for name in &script.scope.var_names {
        if !declared.contains(&name.as_str()) {
            var_names.push(constants.string_str(name)?);
        }
    }
    let function_declarations = script.scope.functions.iter().map(|f| f.0).collect();
    Ok(Code {
        strict: script.strict,
        instructions: body.instructions,
        positions: body.positions,
        handlers: body.handlers,
        locals: resolution.script_locals,
        scopes: body.scopes,
        strings: constants.strings,
        numbers: constants.numbers,
        functions,
        var_names,
        function_declarations,
    })
}

/// The constant pools of a [`Code`], each value once.
#[derive(Default)]
pub(crate) struct Constants {
    strings: Vec<Vec<u16>>,
    string_index: BTreeMap<Vec<u16>, u32>,
    numbers: Vec<f64>,
    number_index: BTreeMap<u64, u32>,
}

impl Constants {
    pub(crate) fn string(&mut self, units: &[u16]) -> Result<u32, CompileError> {
        if let Some(k) = self.string_index.get(units) {
            return Ok(*k);
        }
        let k = u32::try_from(self.strings.len()).map_err(|_| CompileError::TooMany("strings"))?;
        self.strings.push(units.to_vec());
        self.string_index.insert(units.to_vec(), k);
        Ok(k)
    }

    pub(crate) fn string_str(&mut self, s: &str) -> Result<u32, CompileError> {
        let units: Vec<u16> = s.encode_utf16().collect();
        self.string(&units)
    }

    pub(crate) fn number(&mut self, value: f64) -> Result<u32, CompileError> {
        if let Some(k) = self.number_index.get(&value.to_bits()) {
            return Ok(*k);
        }
        let k = u32::try_from(self.numbers.len()).map_err(|_| CompileError::TooMany("numbers"))?;
        self.numbers.push(value);
        self.number_index.insert(value.to_bits(), k);
        Ok(k)
    }
}

/// A jump target inside one template, bound to an instruction index once it is known.
pub(crate) type Label = u32;

/// Whether an exit leaves a loop or switch, or goes to a loop's continue point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum JumpKind {
    Break,
    Continue,
}

/// A statement that `break`, `continue` and `return` look for on the way out.
#[derive(Clone, Debug)]
pub(crate) enum Control {
    /// A loop: `break` goes to `exit`, `continue` to `next`, with `items` operand stack entries
    /// and `envs` environments in place. `labels` is its label set
    /// (`#sec-labelled-statements-runtime-semantics-labelledevaluation`): a `break` or
    /// `continue` with one of them goes to this loop.
    Loop {
        exit: Label,
        next: Label,
        items: u32,
        envs: u32,
        labels: Vec<String>,
    },
    /// A switch statement: `break` goes to `exit`.
    Switch { exit: Label, items: u32, envs: u32 },
    /// A labelled statement that is no loop: a `break` with one of its `labels` goes to `exit`,
    /// where its LabelledEvaluation turns the break completion into a normal one.
    Label {
        labels: Vec<String>,
        exit: Label,
        items: u32,
        envs: u32,
    },
    /// The try block and catch block of a try statement with a `finally` block, entered at
    /// `entry` with a completion record on top of `items` entries.
    Finally {
        entry: Label,
        items: u32,
        envs: u32,
        returns: bool,
        /// The `break` and `continue` exits through it, tag [`TAG_FIRST_JUMP`] onward: the kind
        /// and the index of the target in the control stack.
        jumps: Vec<(JumpKind, usize)>,
    },
}

/// The exits a finished `finally` block dispatches on.
#[derive(Clone, Debug, Default)]
pub(crate) struct Exits {
    returns: bool,
    jumps: Vec<(JumpKind, usize)>,
}

/// A unit of work of the code generator. Expanding a statement or an expression gives the list
/// of tasks it is made of, in order; [`Compiler::run`] keeps them on a stack, so nothing recurses.
#[derive(Clone, Debug)]
pub(crate) enum Task {
    Stmt(StmtId),
    /// A loop with the label set of the labelled statements around it.
    LabelledLoop(StmtId, Vec<String>),
    /// Pushes the value of an expression.
    Expr(ExprId),
    /// Emits an instruction; a jump's target is a [`Label`] until the template is finished.
    Emit(Instruction, Span),
    /// A jump to a label: [`Instruction::LoopJump`] when the label is bound, a forward
    /// [`Instruction::Jump`] otherwise.
    Goto(Label, Span),
    Bind(Label),
    /// The statement-level operand stack entries grow or shrink by one.
    ItemsUp,
    ItemsDown,
    /// The `PushEnv` environments of the frame grow or shrink by one.
    EnvsUp,
    EnvsDown,
    PushControl(Control),
    /// Pops a control entry; a finally's exits are kept under the given key.
    PopControl(Option<u32>),
    /// `break` or `continue` at the current position, with its label if it has one.
    Jump(JumpKind, Option<String>, Span),
    /// Opens an optional chain (`#sec-optional-chains`) whose short-circuit leaves `Short`.
    ChainOpen(Short),
    /// The test of a `?.` in the innermost open chain: the value on top, with `depth - 1` values
    /// of the chain under it, is checked for `undefined` or `null`, which short-circuits the
    /// chain.
    ChainCheck(u32, Span),
    /// Closes the innermost chain: the code its short-circuits jump to.
    ChainClose(Span),
    /// `return` of the value on top of the stack at the current position.
    Return(Span),
    /// The tag dispatch after the `finally` block whose exits were kept under the key.
    FinallyDispatch(u32, Span),
    /// A handler table entry, its labels bound by then.
    Handler {
        kind: HandlerKind,
        start: Label,
        end: Label,
        target: Label,
        items: u32,
        envs: u32,
    },
}

/// What a short-circuited optional chain leaves on the stack
/// (`#sec-optional-chaining-evaluation`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Short {
    /// `undefined`, the value of the chain.
    Undefined,
    /// `true`, the value of `delete` of the chain
    /// (`#sec-delete-operator-runtime-semantics-evaluation`: the chain gives the value
    /// `undefined`, no reference).
    True,
    /// `undefined` as the this value and `undefined` as the function: the chain is the callee of
    /// a call, which then throws a `TypeError`.
    Pair,
}

/// An optional chain being compiled: its short-circuit jumps, each with how many values of the
/// chain are on the stack there.
#[derive(Clone, Debug)]
struct Chain {
    short: Short,
    stubs: Vec<(Label, u32)>,
}

/// The code generator of one template: the script or one function.
pub(crate) struct Compiler<'a> {
    pub(crate) ast: &'a Ast,
    pub(crate) resolution: &'a Resolution,
    pub(crate) constants: &'a mut Constants,
    /// Script code: expression statements store into the completion value register.
    pub(crate) script: bool,
    pub(crate) instructions: Vec<Instruction>,
    pub(crate) positions: Vec<Span>,
    handlers: Vec<Handler>,
    labels: Vec<Option<u32>>,
    /// Statement-level operand stack entries: enumerators of for-in statements and completion
    /// records of `finally` blocks.
    pub(crate) items: u32,
    /// Environments made by `PushEnv` in this frame.
    pub(crate) envs: u32,
    control: Vec<Control>,
    finished: BTreeMap<u32, Exits>,
    next_key: u32,
    tasks: Vec<Task>,
    /// The optional chains being compiled, innermost last.
    chains: Vec<Chain>,
    /// The names of the slots of each environment `PushNamedEnv` makes, as `strings` indices.
    pub(crate) scopes: Vec<Vec<u32>>,
}

/// The finished code of one template.
pub(crate) struct Body {
    pub instructions: Vec<Instruction>,
    pub positions: Vec<Span>,
    pub handlers: Vec<Handler>,
    pub scopes: Vec<Vec<u32>>,
}

pub(crate) fn malformed(what: &str) -> CompileError {
    CompileError::Tree(what.to_string())
}

impl<'a> Compiler<'a> {
    pub(crate) fn new(
        ast: &'a Ast,
        resolution: &'a Resolution,
        constants: &'a mut Constants,
        script: bool,
    ) -> Compiler<'a> {
        Compiler {
            ast,
            resolution,
            constants,
            script,
            instructions: Vec::new(),
            positions: Vec::new(),
            handlers: Vec::new(),
            labels: Vec::new(),
            items: 0,
            envs: 0,
            control: Vec::new(),
            finished: BTreeMap::new(),
            next_key: 0,
            tasks: Vec::new(),
            chains: Vec::new(),
            scopes: Vec::new(),
        }
    }

    pub(crate) fn label(&mut self) -> Label {
        self.labels.push(None);
        (self.labels.len() - 1) as Label
    }

    /// A key for the exits of one `finally` block.
    pub(crate) fn finally_key(&mut self) -> u32 {
        self.next_key = self.next_key.wrapping_add(1);
        self.next_key
    }

    pub(crate) fn string(&mut self, s: &str) -> Result<u32, CompileError> {
        self.constants.string_str(s)
    }

    pub(crate) fn binding(&self, site: Site) -> Result<Binding, CompileError> {
        self.resolution
            .sites
            .get(&site)
            .cloned()
            .ok_or_else(|| malformed("a name the resolver did not see"))
    }

    fn bound(&self, label: Label) -> Option<u32> {
        self.labels.get(label as usize).copied().flatten()
    }

    /// Appends an instruction; a conditional jump must go to a label not bound yet.
    pub(crate) fn emit(
        &mut self,
        instruction: Instruction,
        span: Span,
    ) -> Result<(), CompileError> {
        if let Some(label) = instruction.target()
            && !matches!(instruction, Instruction::LoopJump(_))
            && self.bound(label).is_some()
        {
            return Err(malformed("a forward jump to a bound label"));
        }
        if self.instructions.len() >= u32::MAX as usize {
            return Err(CompileError::TooMany("instructions"));
        }
        self.instructions.push(instruction);
        self.positions.push(span);
        Ok(())
    }

    fn goto(&mut self, label: Label, span: Span) -> Result<(), CompileError> {
        if self.bound(label).is_some() {
            self.emit(Instruction::LoopJump(label), span)
        } else {
            self.emit(Instruction::Jump(label), span)
        }
    }

    fn bind(&mut self, label: Label) -> Result<(), CompileError> {
        let here = u32::try_from(self.instructions.len())
            .map_err(|_| CompileError::TooMany("instructions"))?;
        let slot = self
            .labels
            .get_mut(label as usize)
            .ok_or_else(|| malformed("a label"))?;
        if slot.is_some() {
            return Err(malformed("a label bound twice"));
        }
        *slot = Some(here);
        Ok(())
    }

    /// Runs `tasks` and every task they expand to, in order.
    pub(crate) fn run(&mut self, tasks: Vec<Task>) -> Result<(), CompileError> {
        let base = self.tasks.len();
        self.tasks.extend(tasks.into_iter().rev());
        while self.tasks.len() > base {
            let Some(task) = self.tasks.pop() else { break };
            let expanded = match task {
                Task::Stmt(id) => self.statement(id, Vec::new())?,
                Task::LabelledLoop(id, labels) => self.statement(id, labels)?,
                Task::Expr(id) => self.expression(id)?,
                Task::ChainOpen(short) => {
                    self.chains.push(Chain {
                        short,
                        stubs: Vec::new(),
                    });
                    continue;
                }
                Task::ChainCheck(depth, span) => {
                    self.chain_check(depth, span)?;
                    continue;
                }
                Task::ChainClose(span) => {
                    self.chain_close(span)?;
                    continue;
                }
                Task::Emit(instruction, span) => {
                    self.emit(instruction, span)?;
                    continue;
                }
                Task::Goto(label, span) => {
                    self.goto(label, span)?;
                    continue;
                }
                Task::Bind(label) => {
                    self.bind(label)?;
                    continue;
                }
                Task::ItemsUp => {
                    self.items = self.items.saturating_add(1);
                    continue;
                }
                Task::ItemsDown => {
                    self.items = self.items.saturating_sub(1);
                    continue;
                }
                Task::EnvsUp => {
                    self.envs = self.envs.saturating_add(1);
                    continue;
                }
                Task::EnvsDown => {
                    self.envs = self.envs.saturating_sub(1);
                    continue;
                }
                Task::PushControl(c) => {
                    self.control.push(c);
                    continue;
                }
                Task::PopControl(key) => {
                    if let Some(Control::Finally { returns, jumps, .. }) = self.control.pop()
                        && let Some(key) = key
                    {
                        self.finished.insert(key, Exits { returns, jumps });
                    }
                    continue;
                }
                Task::Jump(kind, label, span) => {
                    self.exit_jump(kind, label.as_deref(), span)?;
                    continue;
                }
                Task::Return(span) => {
                    self.exit_return(span)?;
                    continue;
                }
                Task::FinallyDispatch(key, span) => {
                    self.finally_dispatch(key, span)?;
                    continue;
                }
                Task::Handler {
                    kind,
                    start,
                    end,
                    target,
                    items,
                    envs,
                } => {
                    self.handlers.push(Handler {
                        kind,
                        start,
                        end,
                        target,
                        items,
                        envs,
                    });
                    continue;
                }
            };
            self.tasks.extend(expanded.into_iter().rev());
        }
        Ok(())
    }

    /// `return` of the value on top of the operand stack (`#sec-return-statement`): through the
    /// innermost enclosing `finally` block, or out of the frame.
    fn exit_return(&mut self, span: Span) -> Result<(), CompileError> {
        let (items, envs) = (self.items, self.envs);
        let finally = self.control.iter_mut().rev().find_map(|c| match c {
            Control::Finally {
                entry,
                items,
                envs,
                returns,
                ..
            } => {
                *returns = true;
                Some((*entry, *items, *envs))
            }
            _ => None,
        });
        match finally {
            Some((entry, f_items, f_envs)) => {
                let nip = items.saturating_sub(f_items);
                if nip > 0 {
                    self.emit(Instruction::Nip(nip), span)?;
                }
                for _ in f_envs..envs {
                    self.emit(Instruction::PopEnv, span)?;
                }
                self.emit(Instruction::Tag(TAG_RETURN), span)?;
                self.goto(entry, span)
            }
            None => self.emit(Instruction::Return, span),
        }
    }

    /// `break` or `continue` at the current position: without a label to the innermost loop, or
    /// switch for a `break`; with one to the innermost statement whose label set holds it, a
    /// loop for a `continue` (`#sec-break-statement-runtime-semantics-evaluation`,
    /// `#sec-continue-statement-runtime-semantics-evaluation`, `#sec-loopcontinues`).
    fn exit_jump(
        &mut self,
        kind: JumpKind,
        label: Option<&str>,
        span: Span,
    ) -> Result<(), CompileError> {
        let has = |labels: &[String], l: &str| labels.iter().any(|x| x == l);
        let target = self
            .control
            .iter()
            .rposition(|c| match (c, label) {
                (Control::Loop { .. }, None) => true,
                (Control::Switch { .. }, None) => kind == JumpKind::Break,
                (Control::Loop { labels, .. }, Some(l)) => has(labels, l),
                (Control::Label { labels, .. }, Some(l)) => {
                    kind == JumpKind::Break && has(labels, l)
                }
                _ => false,
            })
            .ok_or_else(|| malformed("a break or continue outside its target"))?;
        self.exit_jump_to(kind, target, span)
    }

    /// A `?.` of the innermost chain: the value on top is `undefined` or `null` exactly when it
    /// is loosely equal to `null` (`#sec-islooselyequal`), and then the chain short-circuits.
    fn chain_check(&mut self, depth: u32, span: Span) -> Result<(), CompileError> {
        let stub = self.label();
        self.emit(Instruction::Dup, span)?;
        self.emit(Instruction::Null, span)?;
        self.emit(Instruction::Eq, span)?;
        self.emit(Instruction::JumpIfTrue(stub), span)?;
        let chain = self
            .chains
            .last_mut()
            .ok_or_else(|| malformed("a ?. outside an optional chain"))?;
        chain.stubs.push((stub, depth));
        Ok(())
    }

    /// The end of the innermost chain: past the code of its short-circuits, one per `?.`, each
    /// of which drops the chain's values and leaves its short value.
    fn chain_close(&mut self, span: Span) -> Result<(), CompileError> {
        let chain = self
            .chains
            .pop()
            .ok_or_else(|| malformed("an optional chain closed twice"))?;
        let end = self.label();
        self.goto(end, span)?;
        let last = chain.stubs.len().saturating_sub(1);
        for (i, (stub, depth)) in chain.stubs.into_iter().enumerate() {
            self.bind(stub)?;
            if depth > 1 {
                self.emit(Instruction::Nip(depth - 1), span)?;
            }
            self.emit(Instruction::Pop, span)?;
            match chain.short {
                Short::Undefined => self.emit(Instruction::Undefined, span)?,
                Short::True => self.emit(Instruction::True, span)?,
                Short::Pair => {
                    self.emit(Instruction::Undefined, span)?;
                    self.emit(Instruction::Undefined, span)?;
                }
            }
            if i < last {
                self.goto(end, span)?;
            }
        }
        self.bind(end)
    }

    /// A `break` or `continue` to the control entry `target`, from the current position: through
    /// the innermost `finally` block between, or straight to the loop or switch.
    fn exit_jump_to(
        &mut self,
        kind: JumpKind,
        target: usize,
        span: Span,
    ) -> Result<(), CompileError> {
        let (items, envs) = (self.items, self.envs);
        let mut through = None;
        for (i, c) in self.control.iter_mut().enumerate().rev() {
            if i <= target {
                break;
            }
            if let Control::Finally {
                entry,
                items,
                envs,
                jumps,
                ..
            } = c
            {
                let index = match jumps.iter().position(|j| *j == (kind, target)) {
                    Some(index) => index,
                    None => {
                        jumps.push((kind, target));
                        jumps.len() - 1
                    }
                };
                let tag = u32::try_from(index)
                    .ok()
                    .and_then(|i| i.checked_add(TAG_FIRST_JUMP))
                    .ok_or(CompileError::TooMany("exits through a finally block"))?;
                through = Some((*entry, *items, *envs, tag));
                break;
            }
        }
        let (label, to_items, to_envs) = match through {
            Some((entry, f_items, f_envs, _)) => (entry, f_items, f_envs),
            None => match self.control.get(target) {
                Some(Control::Loop {
                    exit,
                    next,
                    items,
                    envs,
                    ..
                }) => (
                    if kind == JumpKind::Break {
                        *exit
                    } else {
                        *next
                    },
                    *items,
                    *envs,
                ),
                Some(
                    Control::Switch { exit, items, envs }
                    | Control::Label {
                        exit, items, envs, ..
                    },
                ) => (*exit, *items, *envs),
                _ => return Err(malformed("a jump target")),
            },
        };
        for _ in to_items..items {
            self.emit(Instruction::Pop, span)?;
        }
        for _ in to_envs..envs {
            self.emit(Instruction::PopEnv, span)?;
        }
        if let Some((_, _, _, tag)) = through {
            self.emit(Instruction::Undefined, span)?;
            self.emit(Instruction::Tag(tag), span)?;
        }
        self.goto(label, span)
    }

    /// After a `finally` block, with its completion record on top of the stack: one
    /// [`Instruction::JumpIfTag`] per return, break or continue that went through it, then
    /// [`Instruction::EndFinally`] for normal and throw completions, then the code that goes on
    /// with each of those exits from here.
    fn finally_dispatch(&mut self, key: u32, span: Span) -> Result<(), CompileError> {
        let exits = self.finished.remove(&key).unwrap_or_default();
        let mut stubs: Vec<(Label, Option<(JumpKind, usize)>)> = Vec::new();
        if exits.returns {
            let l = self.label();
            self.emit(
                Instruction::JumpIfTag {
                    tag: TAG_RETURN,
                    target: l,
                },
                span,
            )?;
            stubs.push((l, None));
        }
        for (i, jump) in exits.jumps.iter().enumerate() {
            let tag = u32::try_from(i)
                .ok()
                .and_then(|i| i.checked_add(TAG_FIRST_JUMP))
                .ok_or(CompileError::TooMany("exits through a finally block"))?;
            let l = self.label();
            self.emit(Instruction::JumpIfTag { tag, target: l }, span)?;
            stubs.push((l, Some(*jump)));
        }
        self.emit(Instruction::EndFinally, span)?;
        self.items = self.items.saturating_sub(2);
        if stubs.is_empty() {
            return Ok(());
        }
        let after = self.label();
        self.goto(after, span)?;
        for (l, exit) in stubs {
            self.bind(l)?;
            match exit {
                None => self.exit_return(span)?,
                Some((kind, target)) => {
                    self.emit(Instruction::Pop, span)?;
                    self.exit_jump_to(kind, target, span)?;
                }
            }
        }
        self.bind(after)
    }

    /// The template's code with every label replaced by its instruction index.
    pub(crate) fn finish(self) -> Result<Body, CompileError> {
        let resolve = |label: Label| -> Result<u32, CompileError> {
            self.labels
                .get(label as usize)
                .copied()
                .flatten()
                .ok_or_else(|| malformed("a jump to a label never bound"))
        };
        let mut instructions = Vec::with_capacity(self.instructions.len());
        for instruction in &self.instructions {
            instructions.push(match instruction.target() {
                Some(label) => instruction.with_target(resolve(label)?),
                None => *instruction,
            });
        }
        let mut handlers = Vec::with_capacity(self.handlers.len());
        for h in &self.handlers {
            handlers.push(Handler {
                start: resolve(h.start)?,
                end: resolve(h.end)?,
                target: resolve(h.target)?,
                ..*h
            });
        }
        Ok(Body {
            instructions,
            positions: self.positions,
            handlers,
            scopes: self.scopes,
        })
    }
}
