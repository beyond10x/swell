#![no_std]
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
//! arguments object of a function (`docs/design/engine.md`, decision 3, "Names"), the direct
//! `eval` call and eval code ([`compile_eval`]), whose names are looked up as the same rule
//! says. Slice 6 adds the `let` and `const` declarations, with the scopes, the uninitialized
//! state and the per-iteration bindings of decision 3, "Lexical bindings", and the function
//! declarations of a block, which are bindings of the block, made at its entry, in a script,
//! a function and eval code alike (decision 3, "Function declarations in a block"). A regular
//! expression literal compiles to one instruction that makes a RegExp of its body and flags at
//! each evaluation ([`bytecode::Instruction::NewRegExp`]). Slots and environments are assigned by
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

extern crate alloc;
#[cfg(test)]
extern crate std;

pub mod bytecode;
mod class;
pub mod disasm;
mod expression;
mod function;
mod resolve;
mod statement;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::fmt;

use swell_syntax::Script;
use swell_syntax::ast::{Ast, ExprId, Span, StmtId};
use swell_syntax::ledger::{Ledger, Meter, Stop, Unlimited, map_entry_bytes};

use crate::bytecode::{
    Code, Handler, HandlerKind, Instruction, LexicalName, SlotName, TAG_FIRST_JUMP, TAG_RETURN,
};
use crate::resolve::{Binding, LexicalPlace, Resolution, Site};

/// How the code of a derived constructor, its arrow functions or eval code in it uses the
/// constructor's `this` binding ([`Compiler::this_instruction`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ThisUse {
    /// `this`, and the receiver of a SuperProperty: a checked read.
    Read,
    /// BindThisValue after a SuperCall.
    Bind,
    /// The binding copied into the frame's `this` before a `Return`.
    Keep,
}

/// Why a script did not compile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompileError {
    /// A bound of the format: one function would need more than `u32::MAX` instructions,
    /// constants, slots or another counted thing, named here.
    TooMany(&'static str),
    /// A bound of the format for one class: the private record of a class would need more
    /// slots than an instruction's 16-bit operand numbers (`Instruction::AddPrivateMethod`).
    TooManyInClass(&'static str),
    /// The tree is not one that `swell_syntax::parse_script` returns: an index names no node, a
    /// literal's text is no literal, a `break` stands outside a loop. Never for a parsed script.
    Tree(String),
    /// The ledger refused a charge or its poll read the interrupt; everything the step charged
    /// is credited.
    Stopped(Stop),
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
            CompileError::TooManyInClass(what) => {
                write!(f, "more {what} in one class than the bytecode can number")
            }
            CompileError::Tree(what) => write!(f, "not a tree the parser produces: {what}"),
            CompileError::Stopped(Stop::OverBudget) => {
                write!(f, "the compile step ran out of memory")
            }
            CompileError::Stopped(Stop::Interrupted) => {
                write!(f, "the compile step was interrupted")
            }
        }
    }
}

impl core::error::Error for CompileError {}

impl From<Stop> for CompileError {
    fn from(stop: Stop) -> CompileError {
        CompileError::Stopped(stop)
    }
}

/// Compiles a parsed script into its [`Code`]: the script's own instructions and a template for
/// every function in it.
pub fn compile_script(script: &Script) -> Result<Code, CompileError> {
    compile_script_with(script, &mut Unlimited)
}

/// [`compile_script`] charged to `ledger` (`docs/design/engine.md`, decision 4, "Memory
/// budget"): every growth of the compile step's scratch and of the code is charged before it is
/// made, the scratch is credited before the step returns, and the code carries what its vectors
/// were charged ([`Code::charged`]). The ledger is polled once per
/// [`swell_syntax::ledger::POLL_INTERVAL`] nodes the step visits. A stop answers
/// [`CompileError::Stopped`] with everything credited.
pub fn compile_script_with(script: &Script, ledger: &mut dyn Ledger) -> Result<Code, CompileError> {
    compile(script, (false, false), &[], ledger)
}

/// [`compile_eval`] charged to `ledger`, as [`compile_script_with`], for eval code whose context
/// (`#sec-performeval`, steps 7 to 11) has inDerivedConstructor when `in_derived_constructor`:
/// then `this` in it, its arrow functions included, is the checked lookup of the derived
/// constructor's binding `this` by name, and a SuperCall binds it (`docs/design/engine.md`,
/// "Classes", item 6).
///
/// `private_names` are the positions of the private names of the caller of a direct `eval`
/// (privateIdentifiers of `#sec-evaldeclarationinstantiation`, steps 4 to 7, from the privateEnv
/// PerformEval passes; `docs/design/engine.md`, "Private names", item 4), the names the parse
/// took in its context: each its text (`#x`), the hops from the caller's
/// LexicalEnvironment to the private record that holds its Private Name, and the slot there. A
/// private reference of the eval code that no class of the eval code declares resolves to it
/// with the hops counted as the eval code's own records (the record of its top level, or the
/// empty one PerformEval makes for strict code without one, `#sec-performeval`, steps 17 and
/// 19) plus the caller's hops, and compiles to `GetPrivate`, `SetPrivate` and `PrivateIn` as
/// other code does. None for an indirect `eval`. The list is read during the step and not kept.
pub fn compile_eval_with(
    script: &Script,
    in_derived_constructor: bool,
    private_names: &[(&str, u32, u32)],
    ledger: &mut dyn Ledger,
) -> Result<Code, CompileError> {
    compile(
        script,
        (true, in_derived_constructor),
        private_names,
        ledger,
    )
}

/// Compiles eval code, a script that `swell_syntax::parse_eval` parsed, into its [`Code`]
/// (`#sec-performeval`, step 30 onward). A name the eval code does not bind is looked up by name
/// at run time, through the environments of the code that called `eval` (decision 3, "Names",
/// item 1). [`Code::var_names`] and [`Code::function_declarations`] are the VarDeclaredNames
/// and the function declarations to hoist that `parse_eval` answered in [`Script::scope`], for
/// EvalDeclarationInstantiation (`#sec-evaldeclarationinstantiation`); the code returns its
/// completion value, as a script's does.
pub fn compile_eval(script: &Script) -> Result<Code, CompileError> {
    compile_eval_with(script, false, &[], &mut Unlimited)
}

/// The compile step of a script or of eval code, charged to `ledger`.
///
/// What the step holds is charged as it grows: the code's vectors by capacity, and the
/// scratch (the resolver's structures, the constant maps, the task and control stacks of each
/// template, and the lists of tasks and the small lists each node's expansion builds, by what
/// each growth takes). A list an expansion builds is credited when it drops, once its tasks are
/// on the stack; the rest of the scratch is credited when the step ends, as a whole: on success
/// the ledger then holds exactly what the code's vectors take ([`Code::capacity_bytes`]), which
/// the code carries in [`Code::charged`]; on any error, and at a stop, which wins over every
/// other answer, nothing.
fn compile(
    script: &Script,
    (eval_code, this_by_name): (bool, bool),
    private_names: &[(&str, u32, u32)],
    ledger: &mut dyn Ledger,
) -> Result<Code, CompileError> {
    let meter = Meter::new(ledger);
    let compiled = compile_charged(script, (eval_code, this_by_name), private_names, &meter);
    let compiled = match (compiled, meter.stopped()) {
        (_, Some(stop)) => Err(CompileError::Stopped(stop)),
        (compiled, None) => compiled,
    };
    match compiled {
        Ok(mut code) => {
            let keep = code.capacity_bytes();
            let held = meter.held();
            if held > keep {
                meter.credit(held.saturating_sub(keep));
            } else if meter.charge(keep.saturating_sub(held)).is_err() {
                // The code is made; what its vectors take past what was charged as they grew
                // does not fit.
                drop(code);
                meter.credit_all();
                return Err(CompileError::Stopped(
                    meter.stopped().unwrap_or(Stop::OverBudget),
                ));
            }
            code.charged = keep;
            Ok(code)
        }
        Err(error) => {
            meter.credit_all();
            Err(error)
        }
    }
}

fn compile_charged(
    script: &Script,
    (eval_code, this_by_name): (bool, bool),
    private_names: &[(&str, u32, u32)],
    m: &Meter<'_>,
) -> Result<Code, CompileError> {
    let resolution = resolve::resolve(script, (eval_code, this_by_name), private_names, m)?;
    let mut constants = Constants::default();
    let body = function::compile_script_body(script, &resolution, &mut constants, m)?;
    let mut functions = m.with_capacity(script.ast.functions().len())?;
    for i in 0..script.ast.functions().len() {
        let id = swell_syntax::ast::FunctionId(
            u32::try_from(i).map_err(|_| CompileError::TooMany("functions"))?,
        );
        functions.push(function::compile_function(
            &script.ast,
            id,
            &resolution,
            &mut constants,
            m,
        )?);
    }
    let mut var_names = Vec::new();
    let mut declared: Vec<&str> = m.with_capacity(script.scope.functions.len())?;
    declared.extend(
        script
            .scope
            .functions
            .iter()
            .filter_map(|f| script.ast.function(*f))
            .filter_map(|f| f.name.as_ref().map(|n| n.name.as_str())),
    );
    for name in &script.scope.var_names {
        m.visit()?;
        if !declared.contains(&name.as_str()) {
            let k = constants.string_str(name, m)?;
            m.push(&mut var_names, k)?;
        }
    }
    m.credit(declared.capacity().saturating_mul(size_of::<&str>()));
    drop(declared);
    let mut function_declarations = m.with_capacity(script.scope.functions.len())?;
    function_declarations.extend(script.scope.functions.iter().map(|f| f.0));
    // The lexical names of the top level: the global ones of a script, or those of eval code
    // with their slots and the names of the slots of its record.
    let mut lexical = Vec::new();
    if eval_code {
        for (name, constant, slot) in &resolution.eval_lexical {
            let name = constants.string_str(name, m)?;
            m.push(
                &mut lexical,
                LexicalName {
                    name,
                    constant: *constant,
                    slot: Some(*slot),
                },
            )?;
        }
    } else {
        for l in &script.scope.lexical {
            let name = constants.string_str(&l.name.name, m)?;
            m.push(
                &mut lexical,
                LexicalName {
                    name,
                    constant: l.constant,
                    slot: None,
                },
            )?;
        }
    }
    let mut scopes = body.scopes;
    let eval_scope = if resolution.eval_env.is_empty() {
        None
    } else {
        let index =
            u32::try_from(scopes.len()).map_err(|_| CompileError::TooMany("environments"))?;
        let names = statement::slot_names(&mut constants, &resolution.eval_env, m)?;
        m.push(&mut scopes, names)?;
        Some(index)
    };
    Ok(Code {
        strict: script.strict,
        instructions: body.instructions,
        positions: body.positions,
        handlers: body.handlers,
        locals: resolution.script_locals,
        scopes,
        strings: constants.strings,
        numbers: constants.numbers,
        functions,
        var_names,
        function_declarations,
        lexical,
        eval_scope,
        template_sites: constants.template_sites,
        charged: 0,
    })
}

/// The constant pools of a [`Code`], each value once.
#[derive(Default)]
pub(crate) struct Constants {
    strings: Vec<Vec<u16>>,
    string_index: BTreeMap<Vec<u16>, u32>,
    numbers: Vec<f64>,
    number_index: BTreeMap<u64, u32>,
    /// The site table of the code ([`bytecode::Code::template_sites`]), shared by its templates.
    template_sites: Vec<bytecode::TemplateSite>,
}

impl Constants {
    /// The index of the String constant `units`; a new one is copied twice (the pool and the
    /// map's key), each copy, the map's entry and the pool's growth charged to `m` first.
    pub(crate) fn string(&mut self, units: &[u16], m: &Meter<'_>) -> Result<u32, CompileError> {
        if let Some(k) = self.string_index.get(units) {
            return Ok(*k);
        }
        let k = u32::try_from(self.strings.len()).map_err(|_| CompileError::TooMany("strings"))?;
        m.charge(map_entry_bytes::<Vec<u16>, u32>())?;
        let key = m.copy(units)?;
        let value = m.copy(units)?;
        m.push(&mut self.strings, value)?;
        self.string_index.insert(key, k);
        Ok(k)
    }

    /// [`Constants::string`] of the UTF-16 code units of `s`, made in scratch charged to `m`.
    pub(crate) fn string_str(&mut self, s: &str, m: &Meter<'_>) -> Result<u32, CompileError> {
        let units = m.utf16(s)?;
        let k = self.string(&units, m);
        m.release(units);
        k
    }

    /// The index of the Number constant `value`, the map's entry and the pool's growth charged
    /// to `m` first.
    pub(crate) fn number(&mut self, value: f64, m: &Meter<'_>) -> Result<u32, CompileError> {
        if let Some(k) = self.number_index.get(&value.to_bits()) {
            return Ok(*k);
        }
        let k = u32::try_from(self.numbers.len()).map_err(|_| CompileError::TooMany("numbers"))?;
        m.charge(map_entry_bytes::<u64, u32>())?;
        m.push(&mut self.numbers, value)?;
        self.number_index.insert(value.to_bits(), k);
        Ok(k)
    }

    /// The number of a new tagged template site whose template strings are `strings`, its
    /// entry in the site table with each string a String constant; the entry's lists and the
    /// table's growth charged to `m` first, a visit per string.
    pub(crate) fn template_site(
        &mut self,
        strings: &[swell_syntax::ast::TaggedString],
        m: &Meter<'_>,
    ) -> Result<u32, CompileError> {
        let k = u32::try_from(self.template_sites.len())
            .map_err(|_| CompileError::TooMany("tagged templates"))?;
        let mut cooked = m.with_capacity(strings.len())?;
        let mut raw = m.with_capacity(strings.len())?;
        for string in strings {
            m.visit()?;
            let tv = match &string.cooked {
                Some(units) => Some(self.string(units, m)?),
                None => None,
            };
            cooked.push(tv);
            raw.push(self.string(&string.raw, m)?);
        }
        m.push(
            &mut self.template_sites,
            bytecode::TemplateSite { cooked, raw },
        )?;
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
    /// The try block and catch block of a try statement with a `finally` block, or the step and
    /// the body of a for-of with its IteratorClose (`statement.rs`, "The iterator of a
    /// for-of"), entered at `entry` with a completion record on top of `items` entries.
    Finally {
        entry: Label,
        items: u32,
        envs: u32,
        returns: bool,
        /// The `break` and `continue` exits through it, tag [`TAG_FIRST_JUMP`] onward: the kind
        /// and the index of the target in the control stack.
        jumps: Vec<(JumpKind, usize)>,
        /// It closes the iterator of the for-of loop right under it in the control stack: a
        /// `continue` of that loop goes on to its next step past it (`#sec-loopcontinues`).
        closes: bool,
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
    /// The code from here on is strict mode code, or no longer is: emits
    /// [`Instruction::SetStrict`] and sets [`Compiler::strict`] (the code of a class in
    /// non-strict code, `class.rs`).
    Strict(bool, Span),
    /// A handler table entry, its labels bound by then; its strictness is that of the place
    /// where the task runs ([`Compiler::strict`]).
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
pub(crate) struct Compiler<'a, 'm> {
    pub(crate) ast: &'a Ast,
    /// The compile step's ledger: every growth of the template's code and scratch is charged
    /// to it first.
    pub(crate) m: &'a Meter<'m>,
    pub(crate) resolution: &'a Resolution,
    pub(crate) constants: &'a mut Constants,
    /// Script code: expression statements store into the completion value register.
    pub(crate) script: bool,
    /// Whether the template's code is strict mode code: the script's, eval code's or the
    /// function's own strictness, which the frame that runs it starts with.
    pub(crate) code_strict: bool,
    /// Whether the code being emitted is strict mode code: [`Compiler::code_strict`], or `true`
    /// inside the code of a class ([`Task::Strict`]).
    pub(crate) strict: bool,
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
    /// The names of the slots of each environment `PushNamedEnv` and `PushLexicalEnv` make.
    pub(crate) scopes: Vec<Vec<SlotName>>,
    /// Of a generator template: the local slot of its generator object. Its `return` ends in
    /// [`Instruction::GeneratorReturn`] instead of [`Instruction::Return`].
    pub(crate) generator: Option<u32>,
    /// Of an `async` template: the local slot of its activation. Its `return` ends in
    /// [`Instruction::AsyncReturn`] instead of [`Instruction::Return`]. Of an async generator
    /// template, both are the slot of its async generator object, and its `return` ends in
    /// [`Instruction::AsyncGeneratorReturn`].
    pub(crate) asynchronous: Option<u32>,
    /// Of a derived constructor, an arrow function in one, and eval code whose context has
    /// inDerivedConstructor: where the constructor's `this` binding is ([`Site::This`]), so
    /// every [`Instruction::This`] the template emits becomes its checked read
    /// ([`Compiler::emit`]); `None` elsewhere.
    pub(crate) this_binding: Option<Binding>,
    /// Whether the template is a derived constructor's: `KeepThis` before each `Return`
    /// ([`Compiler::emit_return`]).
    pub(crate) derived: bool,
}

/// The finished code of one template.
pub(crate) struct Body {
    pub instructions: Vec<Instruction>,
    pub positions: Vec<Span>,
    pub handlers: Vec<Handler>,
    pub scopes: Vec<Vec<SlotName>>,
}

/// A list the compile step builds and drops while it runs: every growth is charged to the
/// step's meter before it is made, by the meter's doubling, and its capacity is credited when
/// it drops, on every way out. What its elements own is charged where they are made and passes
/// on with them.
pub(crate) struct List<'a, 'm, T> {
    items: Vec<T>,
    m: &'a Meter<'m>,
}

/// The tasks of one expansion, in order: [`Compiler::run`] moves them onto its task stack.
pub(crate) type Tasks<'a, 'm> = List<'a, 'm, Task>;

impl<'a, 'm, T> List<'a, 'm, T> {
    pub(crate) fn new(m: &'a Meter<'m>) -> List<'a, 'm, T> {
        List {
            items: Vec::new(),
            m,
        }
    }

    /// Push `item`, the list grown first.
    pub(crate) fn push(&mut self, item: T) -> Result<(), CompileError> {
        self.m.push(&mut self.items, item)?;
        Ok(())
    }

    /// Push every item of `items`, the list grown once for all of them first.
    pub(crate) fn extend(
        &mut self,
        items: impl ExactSizeIterator<Item = T>,
    ) -> Result<(), CompileError> {
        self.m.grow_by(&mut self.items, items.len())?;
        self.items.extend(items);
        Ok(())
    }

    /// Push the items of `other`, the list grown first; `other` is credited as it drops here.
    pub(crate) fn append(&mut self, mut other: List<'_, '_, T>) -> Result<(), CompileError> {
        self.m.grow_by(&mut self.items, other.items.len())?;
        self.items.append(&mut other.items);
        Ok(())
    }

    pub(crate) fn len(&self) -> usize {
        self.items.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub(crate) fn as_slice(&self) -> &[T] {
        &self.items
    }

    #[cfg(test)]
    pub(crate) fn capacity(&self) -> usize {
        self.items.capacity()
    }

    /// The items, in order, taken out of the list; its capacity stays charged until it drops.
    pub(crate) fn drain(&mut self) -> alloc::vec::Drain<'_, T> {
        self.items.drain(..)
    }
}

impl<T> Drop for List<'_, '_, T> {
    fn drop(&mut self) {
        self.m
            .credit(self.items.capacity().saturating_mul(size_of::<T>()));
    }
}

/// Drop the exits of a `finally` block and credit their list.
fn release_jumps(m: &Meter<'_>, jumps: Vec<(JumpKind, usize)>) {
    let bytes = jumps
        .capacity()
        .saturating_mul(size_of::<(JumpKind, usize)>());
    drop(jumps);
    m.credit(bytes);
}

pub(crate) fn malformed(what: &str) -> CompileError {
    CompileError::Tree(what.to_string())
}

impl<'a, 'm> Compiler<'a, 'm> {
    pub(crate) fn new(
        ast: &'a Ast,
        resolution: &'a Resolution,
        constants: &'a mut Constants,
        script: bool,
        strict: bool,
        m: &'a Meter<'m>,
    ) -> Compiler<'a, 'm> {
        Compiler {
            ast,
            m,
            resolution,
            constants,
            script,
            code_strict: strict,
            strict,
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
            generator: None,
            asynchronous: None,
            this_binding: None,
            derived: false,
        }
    }

    /// A new label. When the growth of the table is refused the meter has stopped, nothing is
    /// pushed, and the label names no entry; the step ends at its next check.
    pub(crate) fn label(&mut self) -> Label {
        let next = self.labels.len();
        let _ = self.m.push(&mut self.labels, None);
        next as Label
    }

    /// A key for the exits of one `finally` block.
    pub(crate) fn finally_key(&mut self) -> u32 {
        self.next_key = self.next_key.wrapping_add(1);
        self.next_key
    }

    /// Appends the handler `h`, its labels bound by the end of the body, to the table of the
    /// body, the growth charged first: inner handlers come first, as their tasks end first.
    pub(crate) fn handler(&mut self, h: Handler) -> Result<(), CompileError> {
        Ok(self.m.push(&mut self.handlers, h)?)
    }

    /// [`Task::Strict`]: the code from here on is strict, or no longer is.
    pub(crate) fn set_strict(&mut self, strict: bool, span: Span) -> Result<(), CompileError> {
        self.strict = strict;
        self.emit(Instruction::SetStrict(strict), span)
    }

    pub(crate) fn string(&mut self, s: &str) -> Result<u32, CompileError> {
        self.constants.string_str(s, self.m)
    }

    /// The String constant of the code units `units`.
    pub(crate) fn string_units(&mut self, units: &[u16]) -> Result<u32, CompileError> {
        self.constants.string(units, self.m)
    }

    /// The Number constant `value`.
    pub(crate) fn number(&mut self, value: f64) -> Result<u32, CompileError> {
        self.constants.number(value, self.m)
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
        // In a derived constructor, its arrow functions and eval code in it, ResolveThisBinding
        // reads the constructor's binding: `this`, a SuperProperty's receiver, a super call's
        // and `delete super[x]`'s, all emitted as `This` (`docs/design/engine.md`, "Classes",
        // item 6).
        let instruction = match instruction {
            Instruction::This if self.this_binding.is_some() => {
                self.this_instruction(ThisUse::Read)?
            }
            other => other,
        };
        if let Some(label) = instruction.target()
            && !matches!(instruction, Instruction::LoopJump(_))
            && self.bound(label).is_some()
        {
            return Err(malformed("a forward jump to a bound label"));
        }
        if self.instructions.len() >= u32::MAX as usize {
            return Err(CompileError::TooMany("instructions"));
        }
        self.m.push(&mut self.instructions, instruction)?;
        self.m.push(&mut self.positions, span)?;
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

    /// Runs the statements `body` and every task they expand to, in order: a task per statement
    /// goes on the task stack, whose growth is charged first, with no list of its own.
    pub(crate) fn run(&mut self, body: &[StmtId]) -> Result<(), CompileError> {
        let base = self.tasks.len();
        self.m.grow_by(&mut self.tasks, body.len())?;
        self.tasks.extend(body.iter().rev().map(|s| Task::Stmt(*s)));
        while self.tasks.len() > base {
            let Some(task) = self.tasks.pop() else { break };
            // The expansion of a node charges its list of tasks, and the small lists it makes
            // beside it, as they grow; the list is credited once its tasks are on the stack.
            let mut expanded = match task {
                Task::Stmt(id) => {
                    self.m.visit()?;
                    self.statement(id, Vec::new())?
                }
                Task::LabelledLoop(id, labels) => {
                    self.m.visit()?;
                    self.statement(id, labels)?
                }
                Task::Expr(id) => {
                    self.m.visit()?;
                    self.expression(id)?
                }
                Task::ChainOpen(short) => {
                    self.m.push(
                        &mut self.chains,
                        Chain {
                            short,
                            stubs: Vec::new(),
                        },
                    )?;
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
                    self.m.push(&mut self.control, c)?;
                    continue;
                }
                Task::PopControl(key) => {
                    match (self.control.pop(), key) {
                        (Some(Control::Finally { returns, jumps, .. }), Some(key)) => {
                            self.m.charge(map_entry_bytes::<u32, Exits>())?;
                            self.finished.insert(key, Exits { returns, jumps });
                        }
                        (Some(Control::Finally { jumps, .. }), None) => {
                            release_jumps(self.m, jumps)
                        }
                        (Some(Control::Loop { labels, .. } | Control::Label { labels, .. }), _) => {
                            self.m.release(labels)
                        }
                        (Some(Control::Switch { .. }) | None, _) => {}
                    }
                    continue;
                }
                Task::Jump(kind, label, span) => {
                    self.exit_jump(kind, label.as_deref(), span)?;
                    if let Some(label) = label {
                        self.m.release(label);
                    }
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
                Task::Strict(strict, span) => {
                    self.set_strict(strict, span)?;
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
                    self.handler(Handler {
                        kind,
                        start,
                        end,
                        target,
                        items,
                        envs,
                        strict: self.strict,
                    })?;
                    continue;
                }
            };
            self.m.grow_by(&mut self.tasks, expanded.len())?;
            self.tasks.extend(expanded.drain().rev());
            drop(expanded);
            self.m.check()?;
        }
        Ok(())
    }

    /// `return` of the value on top of the operand stack (`#sec-return-statement`): through the
    /// innermost enclosing `finally` block, or out of the frame. A return completion inside
    /// the code of a class (the `return` of a `yield` in a computed key) first puts the code's
    /// own strictness back, since every `finally` block lies outside the class.
    fn exit_return(&mut self, span: Span) -> Result<(), CompileError> {
        if self.strict != self.code_strict {
            self.emit(Instruction::SetStrict(self.code_strict), span)?;
        }
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
            None => match (self.generator, self.asynchronous) {
                (Some(slot), Some(_)) => self.emit(Instruction::AsyncGeneratorReturn(slot), span),
                (Some(slot), None) => self.emit(Instruction::GeneratorReturn(slot), span),
                (None, Some(slot)) => self.emit(Instruction::AsyncReturn(slot), span),
                (None, None) => self.emit_return(span),
            },
        }
    }

    /// `Return`, after `KeepThis` in a derived constructor ("Classes", item 8: the return rule
    /// reads the frame's `this`, which `KeepThis` sets from the binding after every `finally`
    /// block ran).
    pub(crate) fn emit_return(&mut self, span: Span) -> Result<(), CompileError> {
        if self.derived {
            let keep = self.this_instruction(ThisUse::Keep)?;
            self.emit(keep, span)?;
        }
        self.emit(Instruction::Return, span)
    }

    /// The instruction by which the code uses the `this` binding of a derived constructor
    /// ([`Compiler::this_binding`], "Classes", items 6 to 8): a checked read, `BindThis` or
    /// `KeepThis` in the form of its place, a record's hops counted from the LexicalEnvironment
    /// here (the site is at the top of the function, so the environments this frame pushed are
    /// added); in eval code a lookup by name.
    pub(crate) fn this_instruction(&mut self, use_: ThisUse) -> Result<Instruction, CompileError> {
        let binding = self
            .this_binding
            .clone()
            .ok_or_else(|| malformed("a `this` binding outside a derived constructor"))?;
        let out = |hops: u32, envs: u32| {
            hops.checked_add(envs)
                .ok_or(CompileError::TooMany("environment hops"))
        };
        Ok(match (binding, use_) {
            (
                Binding::Lexical {
                    at: LexicalPlace::Local(slot),
                    name,
                    ..
                },
                ThisUse::Read,
            ) => Instruction::GetLocalChecked {
                slot,
                name: self.string(&name)?,
            },
            (
                Binding::Lexical {
                    at: LexicalPlace::Env { hops, slot },
                    ..
                },
                ThisUse::Read,
            ) => Instruction::GetEnvChecked {
                hops: out(hops, self.envs)?,
                slot,
            },
            (
                Binding::Lexical {
                    at: LexicalPlace::Local(slot),
                    ..
                },
                ThisUse::Bind,
            ) => Instruction::BindThis(slot),
            (
                Binding::Lexical {
                    at: LexicalPlace::Env { hops, slot },
                    ..
                },
                ThisUse::Bind,
            ) => Instruction::BindThisEnv {
                hops: out(hops, self.envs)?,
                slot,
            },
            (
                Binding::Lexical {
                    at: LexicalPlace::Local(slot),
                    ..
                },
                ThisUse::Keep,
            ) => Instruction::KeepThis(slot),
            (
                Binding::Lexical {
                    at: LexicalPlace::Env { hops, slot },
                    ..
                },
                ThisUse::Keep,
            ) => Instruction::KeepThisEnv {
                hops: out(hops, self.envs)?,
                slot,
            },
            (Binding::Dynamic(name), ThisUse::Read) => Instruction::GetName(self.string(&name)?),
            (Binding::Dynamic(name), ThisUse::Bind) => {
                Instruction::BindThisName(self.string(&name)?)
            }
            _ => return Err(malformed("a `this` binding")),
        })
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
        let m = self.m;
        let chain = self
            .chains
            .last_mut()
            .ok_or_else(|| malformed("a ?. outside an optional chain"))?;
        m.push(&mut chain.stubs, (stub, depth))?;
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
        let stubs = List {
            items: chain.stubs,
            m: self.m,
        };
        for (i, (stub, depth)) in stubs.items.iter().copied().enumerate() {
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
    /// the innermost `finally` block between, or straight to the loop or switch. A `continue`
    /// of a for-of passes its own IteratorClose by.
    fn exit_jump_to(
        &mut self,
        kind: JumpKind,
        target: usize,
        span: Span,
    ) -> Result<(), CompileError> {
        let (items, envs) = (self.items, self.envs);
        let m = self.m;
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
                closes,
                ..
            } = c
            {
                if *closes && kind == JumpKind::Continue && i == target.saturating_add(1) {
                    continue;
                }
                let index = match jumps.iter().position(|j| *j == (kind, target)) {
                    Some(index) => index,
                    None => {
                        m.push(jumps, (kind, target))?;
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
        let exits = match self.finished.remove(&key) {
            Some(exits) => {
                self.m.credit(map_entry_bytes::<u32, Exits>());
                exits
            }
            None => Exits::default(),
        };
        let jumps = List {
            items: exits.jumps,
            m: self.m,
        };
        let mut stubs: List<'_, '_, (Label, Option<(JumpKind, usize)>)> = List::new(self.m);
        if exits.returns {
            let l = self.label();
            self.emit(
                Instruction::JumpIfTag {
                    tag: TAG_RETURN,
                    target: l,
                },
                span,
            )?;
            stubs.push((l, None))?;
        }
        for (i, jump) in jumps.items.iter().enumerate() {
            let tag = u32::try_from(i)
                .ok()
                .and_then(|i| i.checked_add(TAG_FIRST_JUMP))
                .ok_or(CompileError::TooMany("exits through a finally block"))?;
            let l = self.label();
            self.emit(Instruction::JumpIfTag { tag, target: l }, span)?;
            stubs.push((l, Some(*jump)))?;
        }
        drop(jumps);
        self.emit(Instruction::EndFinally, span)?;
        self.items = self.items.saturating_sub(2);
        if stubs.is_empty() {
            return Ok(());
        }
        let after = self.label();
        self.goto(after, span)?;
        for (l, exit) in stubs.drain() {
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
        // The resolved copies are charged before they are made; the vectors they replace are
        // credited once dropped.
        let m = self.m;
        let mut instructions = m.with_capacity(self.instructions.len())?;
        for instruction in &self.instructions {
            instructions.push(match instruction.target() {
                Some(label) => instruction.with_target(resolve(label)?),
                None => *instruction,
            });
        }
        let mut handlers = m.with_capacity(self.handlers.len())?;
        for h in &self.handlers {
            handlers.push(Handler {
                start: resolve(h.start)?,
                end: resolve(h.end)?,
                target: resolve(h.target)?,
                ..*h
            });
        }
        let replaced = self
            .instructions
            .capacity()
            .saturating_mul(size_of::<Instruction>())
            .saturating_add(
                self.handlers
                    .capacity()
                    .saturating_mul(size_of::<Handler>()),
            );
        // The stacks of the walk are scratch of this body, empty when it ends but for the labels,
        // which own nothing: their capacity is credited with them.
        fn capacity<T>(v: &Vec<T>) -> usize {
            v.capacity().saturating_mul(size_of::<T>())
        }
        let scratch = capacity(&self.tasks)
            .saturating_add(capacity(&self.labels))
            .saturating_add(capacity(&self.control))
            .saturating_add(capacity(&self.chains));
        let Compiler {
            positions, scopes, ..
        } = self;
        m.credit(replaced.saturating_add(scratch));
        Ok(Body {
            instructions,
            positions,
            handlers,
            scopes,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::string::String;
    use std::vec::Vec;
    use std::{format, println, vec};

    use core::cell::Cell;

    use swell_syntax::ledger::{Ledger, Meter, POLL_INTERVAL, Stop, map_entry_bytes};
    use swell_syntax::{ParseOptions, Script, parse_script};

    use super::{CompileError, Compiler, Constants, Task, compile_script_with, resolve};
    use crate::bytecode::{
        Code, FunctionTemplate, Handler, Instruction, LexicalName, Slot, SlotName,
    };
    use swell_syntax::ast::{Span, Stmt};

    /// A ledger that counts what it is asked, refuses a charge past `limit`, and refuses the
    /// charge numbered `refuse_at` (from 1).
    #[derive(Debug, Default)]
    struct Counting {
        charged: usize,
        credited: usize,
        high: usize,
        polls: usize,
        limit: Option<usize>,
        charges: usize,
        refuse_at: Option<usize>,
    }

    impl Counting {
        fn held(&self) -> usize {
            self.charged - self.credited
        }
    }

    impl Ledger for Counting {
        fn charge(&mut self, bytes: usize) -> Result<(), Stop> {
            self.charges += 1;
            if self.refuse_at == Some(self.charges) {
                return Err(Stop::OverBudget);
            }
            let held = self.held() + bytes;
            if self.limit.is_some_and(|limit| held > limit) {
                return Err(Stop::OverBudget);
            }
            self.charged += bytes;
            self.high = self.high.max(held);
            Ok(())
        }

        fn credit(&mut self, bytes: usize) {
            self.credited += bytes;
        }

        fn poll(&mut self) -> Result<(), Stop> {
            self.polls += 1;
            Ok(())
        }
    }

    fn parsed(text: &str) -> Script {
        let source: Vec<u16> = text.encode_utf16().collect();
        parse_script(&source, ParseOptions { max_nesting: 200 }).unwrap()
    }

    fn bytes<T>(capacity: usize) -> usize {
        capacity * size_of::<T>()
    }

    fn scopes_walk(scopes: &Vec<Vec<SlotName>>) -> usize {
        bytes::<Vec<SlotName>>(scopes.capacity())
            + scopes
                .iter()
                .map(|s| bytes::<SlotName>(s.capacity()))
                .sum::<usize>()
    }

    fn template_walk(f: &FunctionTemplate) -> usize {
        bytes::<u16>(f.name.capacity())
            + bytes::<Slot>(f.params.capacity())
            + bytes::<u32>(f.captured.capacity())
            + scopes_walk(&f.scopes)
            + bytes::<Instruction>(f.instructions.capacity())
            + bytes::<Span>(f.positions.capacity())
            + bytes::<Handler>(f.handlers.capacity())
    }

    /// The capacity bytes of every vector of `code`, summed by a walk of its own.
    fn code_walk(code: &Code) -> usize {
        bytes::<Instruction>(code.instructions.capacity())
            + bytes::<Span>(code.positions.capacity())
            + bytes::<Handler>(code.handlers.capacity())
            + scopes_walk(&code.scopes)
            + bytes::<Vec<u16>>(code.strings.capacity())
            + code
                .strings
                .iter()
                .map(|s| bytes::<u16>(s.capacity()))
                .sum::<usize>()
            + bytes::<f64>(code.numbers.capacity())
            + bytes::<FunctionTemplate>(code.functions.capacity())
            + code.functions.iter().map(template_walk).sum::<usize>()
            + bytes::<u32>(code.var_names.capacity())
            + bytes::<u32>(code.function_declarations.capacity())
            + bytes::<LexicalName>(code.lexical.capacity())
    }

    /// A flat script of 20,000 statements `x = 1;`, each four nodes (the statement, the
    /// assignment, the identifier and the number): the compile step polls at least once per
    /// 4,096 of them.
    #[test]
    fn compile_polls_once_per_4096_nodes() {
        let script = parsed(&"x = 1;\n".repeat(20_000));
        let mut ledger = Counting::default();
        let code = compile_script_with(&script, &mut ledger);
        assert!(code.is_ok());
        let floor = 80_000 / POLL_INTERVAL;
        assert!(
            ledger.polls >= floor,
            "{} polls, {floor} wanted",
            ledger.polls
        );
    }

    /// What the ledger holds after the compile step is what the code's vectors take by
    /// capacity, as a walk of its own sums them, and the code says so; a refused charge stops
    /// the step with everything credited.
    #[test]
    fn the_code_carries_the_capacity_bytes_of_its_vectors() {
        let script = parsed(
            r#"
            var a = [1, "two", 3.5], o = { k: a, get g() { return 1; }, m(x) { return x; } };
            let b = 2; const c = `t${b}`;
            function f(p, q) {
                let inner = p;
                try { if (p) throw q; } catch (e) { return function () { return e + inner; }; }
                finally { q = 1; }
                for (let i in o) { label: while (i) { break label; } }
                switch (p) { case 1: return 1; default: return f(p - 1, q); }
            }
            with (o) { k; }
            a?.b?.(c);
            "#,
        );
        let mut ledger = Counting::default();
        let code = compile_script_with(&script, &mut ledger).unwrap();
        let walk = code_walk(&code);
        println!(
            "held {} walk {walk} charged {}",
            ledger.held(),
            code.charged
        );
        assert!(walk > 0);
        assert_eq!(ledger.held(), walk, "held against the walk");
        assert_eq!(code.charged, walk, "the code's own record");
        let mut ledger = Counting {
            limit: Some(walk / 2),
            ..Counting::default()
        };
        let stopped = compile_script_with(&script, &mut ledger);
        assert_eq!(stopped.err(), Some(CompileError::Stopped(Stop::OverBudget)));
        assert_eq!(ledger.held(), 0, "{ledger:?}");
    }

    /// 10,000 distinct string literals and 10,000 distinct numbers: while the compile step runs,
    /// each map of constants holds an entry per value, charged the per-entry constant, and each
    /// string's code units.
    #[test]
    fn the_constant_maps_are_charged_per_entry() {
        let mut text = String::new();
        let mut units = 0;
        for i in 0..10_000 {
            let literal = format!("s{i}");
            units += literal.encode_utf16().count();
            text.push_str(&format!("\"{literal}\"; {}.5;\n", i + 100_000));
        }
        let script = parsed(&text);
        let mut ledger = Counting::default();
        assert!(compile_script_with(&script, &mut ledger).is_ok());
        let floor = 10_000 * map_entry_bytes::<Vec<u16>, u32>()
            + 10_000 * map_entry_bytes::<u64, u32>()
            + 2 * units;
        println!("high-water {} for a floor of {floor}", ledger.high);
        assert!(
            ledger.high >= floor,
            "high-water {} below {floor}",
            ledger.high
        );
    }

    /// A ledger over cells the test reads while the step holds it: what is held and the
    /// high-water mark.
    struct Watch<'c> {
        held: &'c Cell<usize>,
        high: &'c Cell<usize>,
    }

    impl Ledger for Watch<'_> {
        fn charge(&mut self, bytes: usize) -> Result<(), Stop> {
            self.held.set(self.held.get() + bytes);
            self.high.set(self.high.get().max(self.held.get()));
            Ok(())
        }

        fn credit(&mut self, bytes: usize) {
            self.held.set(self.held.get() - bytes);
        }

        fn poll(&mut self) -> Result<(), Stop> {
            Ok(())
        }
    }

    /// A call with 10,000 arguments, the widest node of its script: while the code generator
    /// runs, the ledger's high-water mark above what it held before is at least what the task
    /// stack and the call's expansion hold at once by capacity, and at most twice that plus
    /// 1 KiB. The capacities are read after the run: the task stack keeps the capacity it grew
    /// to, and the call's expansion is made once more in the same way.
    #[test]
    fn a_wide_node_is_charged_near_what_its_lists_hold() {
        let script = parsed(&format!("f({});", vec!["0"; 10_000].join(", ")));
        let (held, high) = (Cell::new(0), Cell::new(0));
        let mut ledger = Watch {
            held: &held,
            high: &high,
        };
        let meter = Meter::new(&mut ledger);
        let resolution = resolve::resolve(&script, (false, false), &[], &meter).unwrap();
        let mut constants = Constants::default();
        let mut c = Compiler::new(
            &script.ast,
            &resolution,
            &mut constants,
            true,
            script.strict,
            &meter,
        );
        let before = held.get();
        high.set(before);
        c.run(&script.body).unwrap();
        let peak = high.get() - before;
        let Some(Stmt::Expression(call)) = script.ast.stmt(script.body[0]).map(|n| &n.stmt) else {
            panic!("the script is no expression statement");
        };
        let widest = c.expression(*call).unwrap();
        let lists = (c.tasks.capacity() + widest.capacity()) * size_of::<Task>();
        println!(
            "peak {peak} above {before}; the task stack {} and the expansion {} tasks, {lists} bytes",
            c.tasks.capacity(),
            widest.capacity()
        );
        assert!(peak >= lists, "peak {peak} below the lists' {lists}");
        assert!(
            peak <= 2 * lists + 1024,
            "peak {peak} above twice the lists' {lists} and 1 KiB"
        );
    }

    /// The private names of the caller of a direct `eval` (`#sec-evaldeclarationinstantiation`,
    /// privateIdentifiers; `story:private-names-in-eval-code`): a private reference the eval
    /// code does not declare resolves to the caller's position, its hops counted as the eval
    /// code's own records (a block's record; the record of its top-level bindings; for strict
    /// code without that one the empty record PerformEval makes, `#sec-performeval`, steps 17
    /// and 19) plus the caller's hops; a name a class of the eval code declares hides the
    /// caller's. Caller: `#x` at 2 hops slot 0, `#y` at 3 hops slot 1. The answer is every
    /// (hops, slot) of `GetPrivate`, `SetPrivate` and `PrivateIn` in the code and its functions.
    #[test]
    fn eval_private_names_count_the_callers_hops() {
        use swell_syntax::ledger::Unlimited;
        use swell_syntax::{EvalContext, parse_eval};
        let texts = ["#x", "#y"];
        let caller = [("#x", 2, 0), ("#y", 3, 1)];
        let places = |text: &str, strict: bool| {
            let context = EvalContext {
                strict_caller: strict,
                in_function: true,
                in_method: true,
                private_names: &texts,
                ..EvalContext::default()
            };
            let source: Vec<u16> = text.encode_utf16().collect();
            let script = parse_eval(&source, context, ParseOptions { max_nesting: 100 })
                .unwrap_or_else(|errors| panic!("{text}: {errors:?}"));
            let code = super::compile_eval_with(&script, false, &caller, &mut Unlimited)
                .unwrap_or_else(|error| panic!("{text}: {error}"));
            let mut found: Vec<(u32, u32)> = code
                .instructions
                .iter()
                .chain(code.functions.iter().flat_map(|f| f.instructions.iter()))
                .filter_map(|i| match i {
                    Instruction::GetPrivate { hops, slot }
                    | Instruction::SetPrivate { hops, slot }
                    | Instruction::PrivateIn { hops, slot } => Some((*hops, *slot)),
                    _ => None,
                })
                .collect();
            found.sort();
            found
        };
        assert_eq!(places("this.#x;", true), vec![(3, 0)]);
        assert_eq!(places("this.#x;", false), vec![(2, 0)]);
        assert_eq!(places("this.#y = 1;", true), vec![(4, 1)]);
        assert_eq!(
            places("let a = 1; (() => a)(); #y in this;", true),
            vec![(4, 1)]
        );
        assert_eq!(
            places("{ let b = 1; (() => b + this.#x)(); }", true),
            vec![(4, 0)]
        );
        assert_eq!(
            places("new (class { #x; m() { return this.#x; } });", true),
            vec![(0, 0)]
        );
    }

    /// A ledger that refuses the n-th charge of the compile step, for every n up to the number of
    /// charges the step makes for a script of every kind of node that builds a list beside its
    /// tasks, ends the step with `Stopped(OverBudget)` and everything credited.
    #[test]
    fn a_refusal_at_any_charge_ends_the_step_with_everything_credited() {
        let script = parsed(
            r#"
            var a = [1, , "two"], o = { k: a, get g() { return 1; }, m(x) { return x; } };
            let b = 2;
            function f(p, q) {
                outer: for (let i in o) {
                    try { if (p) break outer; else continue outer; } finally { q = 1; }
                }
                try { return p; } catch (e) { return e; } finally { q = 2; }
            }
            switch (b) { case 1: a.x++; break; default: a.y ??= `t${b}`; }
            delete a?.b; a?.b?.(b);
            "#,
        );
        let mut ledger = Counting::default();
        assert!(compile_script_with(&script, &mut ledger).is_ok());
        let charges = ledger.charges;
        println!("{charges} charges");
        assert!(charges > 100, "{charges} charges");
        for n in 1..=charges {
            let mut ledger = Counting {
                refuse_at: Some(n),
                ..Counting::default()
            };
            let answer = compile_script_with(&script, &mut ledger);
            assert_eq!(
                answer.err(),
                Some(CompileError::Stopped(Stop::OverBudget)),
                "charge {n}"
            );
            assert_eq!(ledger.held(), 0, "charge {n}: {ledger:?}");
        }
    }
}
