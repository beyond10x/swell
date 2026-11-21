//! Function definitions (15.2, `#sec-function-definitions`): a template per function
//! declaration, function expression and accessor, and the script's own code.
//!
//! - A function's code begins with the instantiation of its function declarations,
//!   FunctionDeclarationInstantiation step 36 (`#sec-functiondeclarationinstantiation`): for the
//!   last declaration of each name, in source order, InstantiateFunctionObject
//!   (`#sec-runtime-semantics-instantiatefunctionobject`) and the store into its binding. The
//!   call itself binds the parameters ([`FunctionTemplate::params`]); `var` bindings start as
//!   `undefined`.
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

use swell_syntax::Script;
use swell_syntax::ast::{Ast, FunctionId, Span};

use crate::bytecode::{FunctionTemplate, Instruction};
use crate::resolve::{Binding, Resolution, Site};
use crate::{Body, CompileError, Compiler, Constants, Task, malformed};

/// The code of the script itself.
pub(crate) fn compile_script_body(
    script: &Script,
    resolution: &Resolution,
    constants: &mut Constants,
) -> Result<Body, CompileError> {
    let mut c = Compiler::new(&script.ast, resolution, constants, true);
    c.run(script.body.iter().map(|s| Task::Stmt(*s)).collect())?;
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
) -> Result<FunctionTemplate, CompileError> {
    let f = ast
        .function(id)
        .ok_or_else(|| malformed("a function index"))?;
    let info = resolution
        .functions
        .get(id.0 as usize)
        .ok_or_else(|| malformed("a function the resolver did not see"))?;
    let name: Vec<u16> = match (&f.name, &info.given_name) {
        (Some(n), _) => n.name.encode_utf16().collect(),
        (None, Some(given)) => given.clone(),
        (None, None) => Vec::new(),
    };
    let mut captured = Vec::with_capacity(info.captured.len());
    for n in &info.captured {
        captured.push(constants.string_str(n)?);
    }
    let length = u32::try_from(f.params.len()).map_err(|_| CompileError::TooMany("parameters"))?;

    let mut c = Compiler::new(ast, resolution, constants, false);
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
    c.run(f.body.iter().map(|s| Task::Stmt(*s)).collect())?;
    c.emit(Instruction::Undefined, f.span)?;
    c.emit(Instruction::Return, f.span)?;
    let body = c.finish()?;
    Ok(FunctionTemplate {
        kind: f.kind,
        name,
        length,
        strict: f.strict,
        params: info.params.clone(),
        locals: info.locals,
        captured,
        self_binding: info.self_binding,
        arguments: info.arguments,
        scopes: body.scopes,
        instructions: body.instructions,
        positions: body.positions,
        handlers: body.handlers,
    })
}
