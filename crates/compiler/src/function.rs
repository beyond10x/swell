//! Function definitions (15.2, `#sec-function-definitions`): a template per function
//! declaration, function expression and accessor, and the script's own code.
//!
//! - A function's code begins with the instantiation of its function declarations,
//!   FunctionDeclarationInstantiation step 36 (`#sec-functiondeclarationinstantiation`): for the
//!   last declaration of each name, in source order, InstantiateFunctionObject
//!   (`#sec-runtime-semantics-instantiatefunctionobject`) and the store into its binding. The
//!   call itself binds the parameters of a simple list ([`FunctionTemplate::params`]); `var`
//!   bindings start as `undefined`.
//! - The code of a function whose parameter list is not simple binds its parameters first
//!   ([`FunctionTemplate::binds_parameters`], `docs/design/engine.md`, decision 3, "A parameter
//!   list that is not simple"): each in order from the arguments the call kept, a default
//!   evaluated only for `undefined`, the rest parameter an Array of the remaining arguments,
//!   then, when a parameter has an initializer, the record of the body's `var` names. Its
//!   `length` is ExpectedArgumentCount (`#sec-static-semantics-expectedargumentcount`).
//! - A script's function declarations are instantiated by GlobalDeclarationInstantiation
//!   (`#sec-globaldeclarationinstantiation`), which the interpreter runs from
//!   [`Code::function_declarations`](crate::bytecode::Code) before the script's code, because it
//!   checks every name before it creates any binding.
//! - A named function expression has a binding of its own name in an environment between the
//!   function and the code around it (`#sec-runtime-semantics-instantiateordinaryfunctionexpression`);
//!   it exists when the function's code uses the name ([`FunctionTemplate::self_binding`]), and
//!   an assignment to it is SetMutableBinding of an immutable binding.
//! - An anonymous function gets the name NamedEvaluation gives it
//!   (`#sec-runtime-semantics-namedevaluation`) in its template: the identifier it is assigned
//!   to with `=`, the `var` it initializes, or the property key of its object literal entry;
//!   an accessor's name is `get <key>` or `set <key>`.
//! - The end of a function's code returns `undefined`; the end of a script returns its
//!   completion value.
//! - A template carries the span the parser gave its function, the function's source text
//!   ([`FunctionTemplate::source_text`]), for `Function.prototype.toString`
//!   (`#sec-function.prototype.tostring`).
//! - An arrow function (`#sec-arrow-function-definitions`) is made by
//!   [`Instruction::ArrowClosure`], which keeps the `this` of the code that runs it. An arrow
//!   function whose own non-strict code calls `eval` directly and that binds no name still gets
//!   a function environment ([`FunctionTemplate::eval_environment`]), the VariableEnvironment
//!   the eval code adds its `var` names to.
//! - An `async` function or arrow function (`#sec-async-function-definitions`,
//!   `#sec-async-arrow-function-definitions`) reserves its last local slot for its activation;
//!   its code starts with [`Instruction::AsyncStart`], before the binding of its parameters, and
//!   from there to the end of its body runs in a handler region that rejects the promise of the
//!   call ([`Instruction::AsyncReject`]); a `return` and the end of the body resolve it
//!   ([`Instruction::AsyncReturn`]). An `async` arrow takes `this`, `arguments` and `new.target`
//!   from its scope as every arrow does.

use swell_syntax::Script;
use swell_syntax::ast::{Ast, BindingTarget, ExprId, Function, FunctionId, Span};
use swell_syntax::ledger::Meter;

use crate::bytecode::{FunctionTemplate, Handler, HandlerKind, Instruction, SlotName, TAG_RETURN};
use crate::resolve::{Binding, FunctionInfo, Resolution, Site};
use crate::{
    Body, Chain, CompileError, Compiler, Constants, Control, Label, Task, malformed, release_jumps,
};

/// The code of the script itself.
pub(crate) fn compile_script_body(
    script: &Script,
    resolution: &Resolution,
    constants: &mut Constants,
    m: &Meter<'_>,
) -> Result<Body, CompileError> {
    let mut c = Compiler::new(&script.ast, resolution, constants, true, m);
    c.run(&script.body)?;
    let end = Span::default();
    c.emit(Instruction::LoadCompletion, end)?;
    c.emit(Instruction::Return, end)?;
    c.finish()
}

/// The template of the function `id`.
pub(crate) fn compile_function(
    ast: &Ast,
    id: FunctionId,
    resolution: &Resolution,
    constants: &mut Constants,
    m: &Meter<'_>,
) -> Result<FunctionTemplate, CompileError> {
    let f = ast
        .function(id)
        .ok_or_else(|| malformed("a function index"))?;
    let info = resolution
        .functions
        .get(id.0 as usize)
        .ok_or_else(|| malformed("a function the resolver did not see"))?;
    let name: Vec<u16> = match (&f.name, &info.given_name) {
        (Some(n), _) => m.utf16(&n.name)?,
        (None, Some(given)) => m.copy(given)?,
        (None, None) => Vec::new(),
    };
    let mut captured = m.with_capacity(info.captured.len())?;
    for n in &info.captured {
        captured.push(constants.string_str(n, m)?);
    }
    let length = u32::try_from(f.expected_argument_count())
        .map_err(|_| CompileError::TooMany("parameters"))?;
    let params = m.copy(&info.params)?;

    let mut c = Compiler::new(ast, resolution, constants, false, m);
    let region = match info.async_slot {
        Some(slot) => Some((slot, async_start(&mut c, f, slot)?)),
        None => None,
    };
    if !f.is_simple() {
        bind_parameters(&mut c, id, info)?;
    }
    c.enter_function_body(id, f.span)?;
    for d in &f.scope.functions {
        let span = ast.function(*d).map(|x| x.span).unwrap_or(f.span);
        let store = match c.binding(Site::Declaration(*d))? {
            Binding::Local(n) => Instruction::SetLocal(n),
            Binding::Env { hops, slot } => Instruction::SetEnv { hops, slot },
            _ => return Err(malformed("a function declaration's binding")),
        };
        c.emit(Instruction::Closure(d.0), span)?;
        c.emit(store, span)?;
        c.emit(Instruction::Pop, span)?;
    }
    match (info.generator_slot, region) {
        (Some(slot), _) => generator_body(&mut c, f, slot)?,
        (None, Some((slot, start))) => async_body(&mut c, f, slot, start)?,
        (None, None) => {
            c.run(&f.body)?;
            c.emit(Instruction::Undefined, f.span)?;
            c.emit(Instruction::Return, f.span)?;
        }
    }
    let body = c.finish()?;
    Ok(FunctionTemplate {
        kind: f.kind,
        name,
        length,
        strict: f.strict,
        generator: info.generator_slot.is_some(),
        asynchronous: info.async_slot.is_some(),
        params,
        binds_parameters: !f.is_simple(),
        parameter_environment: !f.strict && f.scope.parameters_contain_direct_eval,
        locals: info.locals,
        captured,
        self_binding: info.self_binding,
        eval_environment: !f.strict && f.scope.contains_direct_eval && info.captured.is_empty(),
        arguments: info.arguments,
        scopes: body.scopes,
        instructions: body.instructions,
        positions: body.positions,
        handlers: body.handlers,
        source_text: f.span,
    })
}

/// The code of a generator function after FunctionDeclarationInstantiation
/// (`#sec-runtime-semantics-evaluategeneratorbody`; "Suspended frames", item 6,
/// `docs/design/engine.md`): [`Instruction::GeneratorStart`], which makes the generator object
/// into local slot `slot` and suspends, and [`Instruction::EndFinally`] for the normal
/// completion of the first `next`; then the body in a `finally` of its own whose block marks the
/// generator completed ([`Instruction::GeneratorDone`]), so a throw that leaves the body
/// completes it on its way out (GeneratorStart, `#sec-generatorstart`, step 4). The end of the
/// body returns `undefined` as a `return` does: through that `finally`, to
/// [`Instruction::GeneratorReturn`].
fn generator_body(c: &mut Compiler<'_, '_>, f: &Function, slot: u32) -> Result<(), CompileError> {
    let span = f.span;
    c.generator = Some(slot);
    c.emit(Instruction::GeneratorStart(slot), span)?;
    c.emit(Instruction::EndFinally, span)?;
    let (start, entry, returned) = (c.label(), c.label(), c.label());
    let (items, envs) = (c.items, c.envs);
    c.bind(start)?;
    c.m.push(
        &mut c.control,
        Control::Finally {
            entry,
            items,
            envs,
            returns: false,
            jumps: Vec::new(),
            closes: false,
        },
    )?;
    c.run(&f.body)?;
    c.emit(Instruction::Undefined, span)?;
    c.exit_return(span)?;
    if let Some(Control::Finally { jumps, .. }) = c.control.pop() {
        release_jumps(c.m, jumps);
    }
    c.bind(entry)?;
    c.emit(Instruction::GeneratorDone(slot), span)?;
    c.emit(
        Instruction::JumpIfTag {
            tag: TAG_RETURN,
            target: returned,
        },
        span,
    )?;
    c.emit(Instruction::EndFinally, span)?;
    c.bind(returned)?;
    c.emit(Instruction::GeneratorReturn(slot), span)?;
    c.handler(Handler {
        kind: HandlerKind::Finally,
        start,
        end: entry,
        target: entry,
        items,
        envs,
    })
}

/// The start of the code of an `async` function or arrow function, before
/// FunctionDeclarationInstantiation (`#sec-runtime-semantics-evaluateasyncfunctionbody`, step 1;
/// "`async` functions on suspended frames", item 1, `docs/design/engine.md`):
/// [`Instruction::AsyncStart`], which makes the promise of the call and the activation into local
/// slot `slot`; answers the label of the region after it, which [`async_body`] closes.
fn async_start(c: &mut Compiler<'_, '_>, f: &Function, slot: u32) -> Result<Label, CompileError> {
    c.asynchronous = Some(slot);
    c.emit(Instruction::AsyncStart(slot), f.span)?;
    let start = c.label();
    c.bind(start)?;
    Ok(start)
}

/// The rest of the code of an `async` function or arrow function after the instantiation of its
/// function declarations: the body, whose end returns `undefined` as a `return` does, through
/// [`Instruction::AsyncReturn`] (AsyncBlockStart, `#sec-asyncblockstart`, steps 2.e and 2.f).
/// Everything from the label `start`, the binding of the parameters included, is a
/// [`HandlerKind::Catch`] region whose block is [`Instruction::AsyncReject`], so a throw that
/// leaves the body (step 2.g), or a default of a parameter that throws
/// (EvaluateAsyncFunctionBody, step 3), rejects the promise of the call, which the call returns.
fn async_body(
    c: &mut Compiler<'_, '_>,
    f: &Function,
    slot: u32,
    start: Label,
) -> Result<(), CompileError> {
    let span = f.span;
    c.run(&f.body)?;
    c.emit(Instruction::Undefined, span)?;
    c.exit_return(span)?;
    let rejected = c.label();
    c.bind(rejected)?;
    c.emit(Instruction::AsyncReject(slot), span)?;
    c.handler(Handler {
        kind: HandlerKind::Catch,
        start,
        end: rejected,
        target: rejected,
        items: 0,
        envs: 0,
    })
}

/// The binding of the parameters of function `id`, whose list is not simple, from the
/// arguments the call kept after the frame's `locals` slots
/// (`#sec-functiondeclarationinstantiation`, step 26, IteratorBindingInitialization of the
/// FormalParameters with the list iterator of the arguments,
/// `#sec-runtime-semantics-iteratorbindinginitialization`): each parameter in order, its
/// argument, or for `undefined` the value of its initializer, or the rest Array; then, when a
/// parameter has an initializer, the record of the body's `var` names, which becomes the
/// VariableEnvironment, and the `var` names a parameter or `arguments` also binds set to its
/// value (steps 27 and 28).
fn bind_parameters(
    c: &mut Compiler<'_, '_>,
    id: FunctionId,
    info: &FunctionInfo,
) -> Result<(), CompileError> {
    let ast = c.ast;
    let f = ast
        .function(id)
        .ok_or_else(|| malformed("a function index"))?;
    let last = f.params.len().saturating_sub(1);
    for (i, p) in f.params.iter().enumerate() {
        let n = u32::try_from(i).map_err(|_| CompileError::TooMany("parameters"))?;
        let at = info
            .locals
            .checked_add(n)
            .ok_or(CompileError::TooMany("local slots"))?;
        let span = p.span();
        if f.rest && i == last {
            c.emit(Instruction::RestArguments(at), span)?;
        } else {
            c.emit(Instruction::Argument(at), span)?;
            if let Some(e) = f.initializer(i) {
                let given = c.label();
                c.emit(Instruction::JumpIfNotUndefined(given), span)?;
                expression_value(c, e)?;
                c.bind(given)?;
            }
        }
        // A pattern destructures the value into the names it binds (BindingInitialization,
        // `#sec-runtime-semantics-bindinginitialization`).
        if let Some(pattern) = p.pattern() {
            expression_value(c, pattern)?;
            c.emit(Instruction::Pop, span)?;
            continue;
        }
        let store = match c.binding(Site::Param(id, n))? {
            Binding::Local(slot) => Instruction::SetLocal(slot),
            Binding::Env { hops, slot } => Instruction::SetEnv { hops, slot },
            _ => return Err(malformed("a parameter's binding")),
        };
        c.emit(store, span)?;
        c.emit(Instruction::Pop, span)?;
    }
    if !f.has_parameter_expressions() {
        return Ok(());
    }
    let pushed = match &info.var_env {
        Some(names) => {
            let mut table = c.m.with_capacity(names.len())?;
            for name in names {
                let k = c.string(name)?;
                table.push(SlotName::plain(k).ok_or(CompileError::TooMany("strings"))?);
            }
            let index =
                u32::try_from(c.scopes.len()).map_err(|_| CompileError::TooMany("environments"))?;
            c.m.push(&mut c.scopes, table)?;
            c.emit(Instruction::PushVarEnv(index), f.span)?;
            c.envs = c.envs.saturating_add(1);
            1
        }
        None => 0,
    };
    for (j, name) in f.scope.var_names.iter().enumerate() {
        let n = u32::try_from(j).map_err(|_| CompileError::TooMany("var names"))?;
        let resolution = c.resolution;
        let Some(target) = resolution.sites.get(&Site::BodyVar(id, n)) else {
            continue;
        };
        let source = parameter_site(f, id, name)?.unwrap_or(Site::Arguments(id));
        let load = match c.binding(source)? {
            Binding::Local(slot) => Instruction::GetLocal(slot),
            Binding::Env { hops, slot } => Instruction::GetEnv {
                hops: hops.saturating_add(pushed),
                slot,
            },
            _ => return Err(malformed("a parameter's binding")),
        };
        let store = match target {
            Binding::Local(slot) => Instruction::SetLocal(*slot),
            Binding::Env { hops, slot } => Instruction::SetEnv {
                hops: *hops,
                slot: *slot,
            },
            _ => return Err(malformed("a var binding of the body")),
        };
        c.emit(load, f.span)?;
        c.emit(store, f.span)?;
        c.emit(Instruction::Pop, f.span)?;
    }
    Ok(())
}

/// The site of the last parameter of `f`, function `id`, that binds `name`: the parameter, or
/// the name in its pattern; `None` when none does.
fn parameter_site(f: &Function, id: FunctionId, name: &str) -> Result<Option<Site>, CompileError> {
    for (k, p) in f.params.iter().enumerate().rev() {
        match p {
            BindingTarget::Identifier(n) if n.name == name => {
                let k = u32::try_from(k).map_err(|_| CompileError::TooMany("parameters"))?;
                return Ok(Some(Site::Param(id, k)));
            }
            BindingTarget::Identifier(_) => {}
            BindingTarget::Pattern(pattern) => {
                if let Some(b) = pattern.names.iter().rev().find(|b| b.name.name == name) {
                    return Ok(Some(Site::Bound(b.pattern, b.property)));
                }
            }
        }
    }
    Ok(None)
}

/// The value of the expression `e` onto the operand stack: its tasks run as
/// [`Compiler::run`] runs a statement's, of the kinds an expression expands to
/// (`expression.rs`), each expansion charged as it is made.
fn expression_value(c: &mut Compiler<'_, '_>, e: ExprId) -> Result<(), CompileError> {
    let base = c.tasks.len();
    c.m.push(&mut c.tasks, Task::Expr(e))?;
    while c.tasks.len() > base {
        let Some(task) = c.tasks.pop() else { break };
        match task {
            Task::Expr(id) => {
                c.m.visit()?;
                let mut expanded = c.expression(id)?;
                c.m.grow_by(&mut c.tasks, expanded.len())?;
                c.tasks.extend(expanded.drain().rev());
                drop(expanded);
                c.m.check()?;
            }
            Task::Emit(instruction, span) => c.emit(instruction, span)?,
            Task::Goto(label, span) => c.goto(label, span)?,
            Task::Bind(label) => c.bind(label)?,
            Task::ChainOpen(short) => c.m.push(
                &mut c.chains,
                Chain {
                    short,
                    stubs: Vec::new(),
                },
            )?,
            Task::ChainCheck(depth, span) => c.chain_check(depth, span)?,
            Task::ChainClose(span) => c.chain_close(span)?,
            Task::Handler {
                kind,
                start,
                end,
                target,
                items,
                envs,
            } => c.handler(Handler {
                kind,
                start,
                end,
                target,
                items,
                envs,
            })?,
            _ => return Err(malformed("a task no expression expands to")),
        }
    }
    Ok(())
}
