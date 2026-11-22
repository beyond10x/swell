//! Scripts (ECMA-262 16.1, `#sec-scripts`): the Script Record (`#sec-script-records`: its realm
//! and its compiled code with their constants), ParseScript (`#sec-parse-script`),
//! ScriptEvaluation (`#sec-runtime-semantics-scriptevaluation`) and
//! GlobalDeclarationInstantiation (`#sec-globaldeclarationinstantiation`). Filled by
//! `story:interpreter-and-script-entry-points`.
//!
//! ParseScript parses with `swell-syntax`, whose depth `parser_nesting` bounds (decision 2), and
//! compiles with `swell-compiler`, which walks only trees the parser accepted and recurses on no
//! Rust stack. Its answers: a Script Record, or the parse errors as `SyntaxError` objects of the
//! realm given (the message of each, with its line and column). A compile error is not a parse
//! error of the draft: `CompileError::TooMany` (a bound of the bytecode format) is answered as
//! one `SyntaxError` naming the bound, an implementation limit like `parser_nesting`;
//! `CompileError::Tree` (a tree the parser cannot produce) is a broken engine invariant and ends
//! the call in `Terminated(EngineFault)`. ParseScript runs no code, calls no hook, spends no step
//! and reads no interrupt itself: `parse_script` and `cx.parse_script` read the flag before they
//! call it (HOST-166, HOST-167).
//!
//! The record owns the compiled code and the atoms of its String constants, of their property
//! keys and of its functions' names; the code and the record are charged to the memory budget
//! when it is made, the atoms as they are interned, and the sweep that frees the record credits
//! them. The source's UTF-16 copy is charged from the start of the parse; the parser's tree is
//! not, since `swell-syntax` allocates it with no access to the budget (its size is bounded by
//! the source's).
//!
//! Source text (`docs/design/engine.md`, decision 4, "Source text."): the copy the parse read is
//! kept with the compiled code ([`Compiled::source`]) and its charge becomes part of the
//! record's ([`code_bytes`]) when the parse and compile step ends, two bytes per code unit; the
//! sweep that frees the record credits it with the code, never earlier. The record lives while
//! the host holds the script, while its code runs and while a function made from it is
//! reachable (each reports its [[ScriptOrModule]]), so the text does too, and
//! `Function.prototype.toString` reads a function's [[SourceText]] out of it
//! (`builtins/function.rs`). A parse or compile step that makes no code credits the copy
//! ([`Agent::credit_source_copy`]). Eval code and the `Function` constructor keep theirs the same
//! way (`dynamic_code.rs`).
//!
//! Kind site of the collector: [`ScriptRecord::trace`] (the realm and the constants' atoms).

use std::rc::Rc;

use swell_compiler::CompileError;
use swell_compiler::bytecode::{Code, Handler, Instruction};
use swell_syntax::ParseOptions;
use swell_syntax::ast::Span;

use crate::heap::arena::Handle;
use crate::heap::collect::{Item, Mark};
use crate::host::agent::Agent;
use crate::host::host_object::{Brand, HostData, HostObject, Tracer};
use crate::host::outcome::{Abrupt, Completion, Termination};
use crate::host::{ErrorKind, Host, HostDefined, Source};
use crate::interp::frame::{Frame, frame_bytes};
use crate::object::property::{Key, array_index};
use crate::value::{StringRef, Value};

/// A script's compiled code, the source it was compiled from and the atoms of its constants.
pub(crate) struct Compiled {
    pub(crate) code: Code,
    /// The code units the parse read: the source of a script, the String of eval code, the
    /// whole text of the last parse of CreateDynamicFunction (decision 4, "Source text.").
    /// Each template's `source_text` is a span of it.
    pub(crate) source: Vec<u16>,
    /// The atom of each String constant, by index.
    pub(crate) strings: Vec<StringRef>,
    /// The property key of each String constant: its array index or its atom.
    pub(crate) keys: Vec<Key>,
    /// The atom of each function template's name (the value SetFunctionName gives).
    pub(crate) names: Vec<StringRef>,
    /// The property key of each function template's name.
    pub(crate) name_keys: Vec<Key>,
}

/// The `[[HostDefined]]` value of a script, kept as the data of a [`HostObject`] so that its
/// `trace` is called the one way host data is.
struct ScriptHostDefined(HostDefined);

impl HostData for ScriptHostDefined {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        self.0.trace(tracer);
    }
}

/// A Script Record (16.1.4, Table 37): [[Realm]], [[ECMAScriptCode]] as compiled code, and
/// [[HostDefined]]. [[LoadedModules]] comes with modules (slice 11). Every field is `Some` for a
/// record made by ParseScript; the record of the tests of the collector has none.
pub(crate) struct ScriptRecord {
    pub(crate) realm: Option<Handle>,
    pub(crate) compiled: Option<Rc<Compiled>>,
    host_defined: Option<HostObject>,
}

impl ScriptRecord {
    /// Kind site of the collector: [[Realm]], the atoms of the code's constants and of its
    /// functions' names (the property keys are the same atoms), and [[HostDefined]].
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("script.rs ScriptRecord::trace");
        if let Some(realm) = self.realm {
            mark.push(Item::Realm(realm));
        }
        if let Some(compiled) = &self.compiled {
            for &string in compiled.strings.iter().chain(&compiled.names) {
                mark.value(Value::String(string));
            }
        }
        if let Some(host_defined) = &self.host_defined {
            host_defined.trace(mark);
        }
    }

    /// The host-defined value.
    pub(crate) fn host_defined(&self) -> Option<HostDefined> {
        let data: &dyn std::any::Any = self.host_defined.as_ref()?.data.as_deref()?;
        data.downcast_ref::<ScriptHostDefined>()
            .map(|held| HostDefined::clone(&held.0))
    }

    /// A Script Record, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> ScriptRecord {
        ScriptRecord {
            realm: None,
            compiled: None,
            host_defined: None,
        }
    }
}

/// What ParseScript answers inside the engine.
pub(crate) enum ParseAnswer {
    /// The Script Record.
    Script(Handle),
    /// The `SyntaxError` objects, at least one.
    Errors(Vec<Value>),
}

/// The bytes a `Vec` of `len` elements of `T` owns.
pub(crate) fn vec_bytes<T>(len: usize) -> u64 {
    u64::try_from(len.saturating_mul(size_of::<T>())).unwrap_or(u64::MAX)
}

/// The code units of a source copied for its parse, and the bytes charged for them: kept with
/// the code made from them ([`Agent::make_script_record`]), or credited
/// ([`Agent::credit_source_copy`]) when the step makes no code.
pub(crate) struct SourceCopy {
    pub(crate) units: Vec<u16>,
    pub(crate) charged: u64,
}

/// The bytes of the names tables of the environments `PushNamedEnv` makes (decision 3, "Names":
/// they are charged with the compiled code).
fn scopes_bytes(scopes: &[Vec<u32>]) -> u64 {
    scopes
        .iter()
        .fold(vec_bytes::<Vec<u32>>(scopes.len()), |bytes, names| {
            bytes.saturating_add(vec_bytes::<u32>(names.len()))
        })
}

/// The bytes the compiled code owns, charged with its record: the code, its constants and
/// templates, and the `source_units` code units of the source kept with it (decision 4, "Source
/// text.").
pub(crate) fn code_bytes(code: &Code, source_units: usize) -> u64 {
    let mut bytes = vec_bytes::<u16>(source_units)
        .saturating_add(vec_bytes::<Instruction>(code.instructions.len()))
        .saturating_add(vec_bytes::<Span>(code.positions.len()))
        .saturating_add(vec_bytes::<Handler>(code.handlers.len()))
        .saturating_add(vec_bytes::<f64>(code.numbers.len()))
        .saturating_add(vec_bytes::<u32>(code.var_names.len()))
        .saturating_add(vec_bytes::<u32>(code.function_declarations.len()))
        .saturating_add(scopes_bytes(&code.scopes));
    for string in &code.strings {
        bytes = bytes
            .saturating_add(vec_bytes::<u16>(string.len()))
            .saturating_add(vec_bytes::<Vec<u16>>(1))
            .saturating_add(vec_bytes::<StringRef>(1))
            .saturating_add(vec_bytes::<Key>(1));
    }
    for function in &code.functions {
        bytes = bytes
            .saturating_add(vec_bytes::<swell_compiler::bytecode::FunctionTemplate>(1))
            .saturating_add(vec_bytes::<u16>(function.name.len()))
            .saturating_add(vec_bytes::<StringRef>(1))
            .saturating_add(vec_bytes::<Key>(1))
            .saturating_add(vec_bytes::<swell_compiler::bytecode::Slot>(
                function.params.len(),
            ))
            .saturating_add(vec_bytes::<u32>(function.captured.len()))
            .saturating_add(scopes_bytes(&function.scopes))
            .saturating_add(vec_bytes::<Instruction>(function.instructions.len()))
            .saturating_add(vec_bytes::<Span>(function.positions.len()))
            .saturating_add(vec_bytes::<Handler>(function.handlers.len()));
    }
    bytes.saturating_add(vec_bytes::<Compiled>(1))
}

impl<H: Host> Agent<H> {
    /// The atom of `units` and its property key.
    pub(crate) fn atom_and_key(&mut self, units: &[u16]) -> Completion<(StringRef, Key)> {
        let atom = match self.heap.intern(units) {
            Ok(atom) => atom,
            Err(_) => return Err(self.terminate(Termination::MemoryBudget)),
        };
        let key = match array_index(units) {
            Some(index) => Key::Index(index),
            None => Key::String(atom),
        };
        Ok((atom, key))
    }

    /// ParseScript (`#sec-parse-script`) of `source` in `realm`, with `host_defined` as its
    /// [[HostDefined]]. Over the memory budget the run ends in `Terminated(MemoryBudget)`.
    pub(crate) fn parse_script_in(
        &mut self,
        realm: Handle,
        source: Source<'_>,
        host_defined: Option<HostDefined>,
    ) -> Completion<ParseAnswer> {
        let options = ParseOptions {
            max_nesting: self.limits.parser_nesting,
        };
        let copy = match source {
            Source::Utf16(units) => {
                self.source_copy(units.len(), |copy| copy.extend_from_slice(units))?
            }
            Source::Utf8(text) => {
                let length = text.encode_utf16().count();
                self.source_copy(length, |copy| copy.extend(text.encode_utf16()))?
            }
        };
        let script = match swell_syntax::parse_script(&copy.units, options) {
            Ok(script) => script,
            Err(errors) => {
                self.credit_source_copy(copy);
                return self.syntax_errors(realm, errors).map(ParseAnswer::Errors);
            }
        };
        let compiled = swell_compiler::compile_script(&script);
        drop(script);
        let (code, copy) = match self.compiled_with_source(realm, compiled, copy)? {
            Ok(made) => made,
            Err(error) => return Ok(ParseAnswer::Errors(vec![error])),
        };
        self.make_script_record(realm, code, copy, host_defined)
            .map(ParseAnswer::Script)
    }

    /// A copy of the `length` code units `fill` appends, charged to the memory budget first
    /// (decision 4, "Source text."): over the budget, or when the allocator refuses, the run ends
    /// in `Terminated(MemoryBudget)` and nothing stays charged.
    pub(crate) fn source_copy(
        &mut self,
        length: usize,
        fill: impl FnOnce(&mut Vec<u16>),
    ) -> Completion<SourceCopy> {
        let charged = vec_bytes::<u16>(length);
        if self.heap.accounting.charge(charged).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        let mut units = Vec::new();
        if units.try_reserve_exact(length).is_err() {
            self.heap.accounting.credit(charged);
            return Err(self.terminate(Termination::MemoryBudget));
        }
        fill(&mut units);
        Ok(SourceCopy { units, charged })
    }

    /// The end of a parse or compile step that made no code: the copy of its source is dropped
    /// and its bytes credited.
    pub(crate) fn credit_source_copy(&mut self, copy: SourceCopy) {
        let SourceCopy { units, charged } = copy;
        drop(units);
        self.heap.accounting.credit(charged);
    }

    /// [`Agent::compiled_code`] of the compile step of the source whose copy is `copy`: the code
    /// with the copy; on any other answer the copy is credited.
    pub(crate) fn compiled_with_source(
        &mut self,
        realm: Handle,
        compiled: Result<Code, CompileError>,
        copy: SourceCopy,
    ) -> Completion<Result<(Code, SourceCopy), Value>> {
        match self.compiled_code(realm, compiled) {
            Ok(Ok(code)) => Ok(Ok((code, copy))),
            Ok(Err(error)) => {
                self.credit_source_copy(copy);
                Ok(Err(error))
            }
            Err(abrupt) => {
                self.credit_source_copy(copy);
                Err(abrupt)
            }
        }
    }

    /// The `SyntaxError` objects of `realm` for the errors of a parse, each message with its
    /// line and column; each refusal among them is counted ([`Agent::refusals`]). Every parse
    /// of the engine answers its errors here: `parse_script`, `eval` and `Function`.
    pub(crate) fn syntax_errors(
        &mut self,
        realm: Handle,
        errors: Vec<swell_syntax::SyntaxError>,
    ) -> Completion<Vec<Value>> {
        let mut objects = Vec::new();
        for error in errors {
            self.count_refusal(&error.message);
            let message = format!("{} ({}:{})", error.message, error.line, error.column);
            let object = self.new_error_in(realm, ErrorKind::SyntaxError, &message)?;
            objects.push(Value::Object(object));
        }
        Ok(objects)
    }

    /// The answer of the compile step: the code, or for a bound of the bytecode format the
    /// `SyntaxError` of `realm` naming it; a tree the parser cannot produce faults the agent.
    pub(crate) fn compiled_code(
        &mut self,
        realm: Handle,
        compiled: Result<Code, CompileError>,
    ) -> Completion<Result<Code, Value>> {
        match compiled {
            Ok(code) => Ok(Ok(code)),
            Err(error @ CompileError::TooMany(_)) => {
                let message = error.to_string();
                let object = self.new_error_in(realm, ErrorKind::SyntaxError, &message)?;
                Ok(Err(Value::Object(object)))
            }
            Err(CompileError::Tree(_)) => Err(self.fault()),
        }
    }

    /// The Script Record of `code` in `realm`, with its atoms interned and the source `copy` it
    /// was compiled from kept with it: the copy's charge becomes part of the record's, which the
    /// sweep that frees the record credits (decision 4, "Source text.").
    pub(crate) fn make_script_record(
        &mut self,
        realm: Handle,
        code: Code,
        copy: SourceCopy,
        host_defined: Option<HostDefined>,
    ) -> Completion<Handle> {
        // The copy is charged again below with the code, as a part of the record; no safe point
        // comes between, and on every way out after this the copy is dropped uncharged.
        let SourceCopy {
            units: source,
            charged,
        } = copy;
        self.heap.accounting.credit(charged);
        let owned = code_bytes(&code, source.len());
        if self.heap.accounting.check(owned).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        // The atoms are reported by no site until the record holds them; no safe point comes
        // before it does.
        let mut strings = Vec::with_capacity(code.strings.len());
        let mut keys = Vec::with_capacity(code.strings.len());
        for units in &code.strings {
            let (atom, key) = self.atom_and_key(units)?;
            strings.push(atom);
            keys.push(key);
        }
        let mut names = Vec::with_capacity(code.functions.len());
        let mut name_keys = Vec::with_capacity(code.functions.len());
        for function in &code.functions {
            let (atom, key) = self.atom_and_key(&function.name)?;
            names.push(atom);
            name_keys.push(key);
        }
        let record = ScriptRecord {
            realm: Some(realm),
            compiled: Some(Rc::new(Compiled {
                code,
                source,
                strings,
                keys,
                names,
                name_keys,
            })),
            host_defined: host_defined.map(|value| HostObject {
                brand: Brand(0),
                data: Some(Box::new(ScriptHostDefined(value))),
            }),
        };
        match crate::heap::store(
            &mut self.heap.accounting,
            &mut self.heap.scripts,
            record,
            owned,
        ) {
            Ok(handle) => Ok(handle),
            Err(_) => Err(self.terminate(Termination::MemoryBudget)),
        }
    }

    /// The realm and the code of the script record `script`.
    pub(crate) fn script_parts(&mut self, script: Handle) -> Completion<(Handle, Rc<Compiled>)> {
        let parts = match self.heap.scripts.get(script) {
            Ok(ScriptRecord {
                realm: Some(realm),
                compiled: Some(compiled),
                ..
            }) => Some((*realm, Rc::clone(compiled))),
            _ => None,
        };
        match parts {
            Some(parts) => Ok(parts),
            None => Err(self.fault()),
        }
    }

    /// ScriptEvaluation (`#sec-runtime-semantics-scriptevaluation`): the script's frame, with
    /// the script's realm as its Realm and the global environment as its LexicalEnvironment, then
    /// GlobalDeclarationInstantiation, then the code, whose completion value is the result. When
    /// code is running it is a re-entry (`interp/mod.rs`).
    pub(crate) fn script_evaluation(&mut self, script: Handle) -> Completion<Value> {
        let (realm, compiled) = self.script_parts(script)?;
        let entered = self.enter_code(None)?;
        let result = self.run_script_frame(script, realm, compiled);
        self.leave_code(entered);
        result
    }

    fn run_script_frame(
        &mut self,
        script: Handle,
        realm: Handle,
        compiled: Rc<Compiled>,
    ) -> Completion<Value> {
        self.check_frame_depth()?;
        self.step()?;
        let this = self.global_this_binding(realm)?;
        let count = usize::try_from(compiled.code.locals).unwrap_or(usize::MAX);
        let charged = frame_bytes().saturating_add(vec_bytes::<Value>(count));
        if self.heap.accounting.charge(charged).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        let mut locals = Vec::new();
        if locals.try_reserve_exact(count).is_err() {
            self.heap.accounting.credit(charged);
            return Err(self.terminate(Termination::MemoryBudget));
        }
        locals.resize(count, Value::Undefined);
        let strict = compiled.code.strict;
        let frame = Frame {
            compiled: Rc::clone(&compiled),
            template: None,
            pc: 0,
            stack: Vec::new(),
            locals,
            env: None,
            var_env: None,
            pushed_envs: 0,
            this,
            function: None,
            realm,
            script,
            strict,
            completion: Value::Undefined,
            constructed: None,
            charged,
        };
        self.run_frame(frame, |agent| {
            agent.global_declaration_instantiation(&compiled, script, realm)
        })
    }

    /// GlobalDeclarationInstantiation (`#sec-globaldeclarationinstantiation`) of a script of
    /// slice 1, which has no lexical declaration (steps 3 and 4 have nothing to check, step 15
    /// nothing to create), with the compiler's `function_declarations` as functionsToInitialize
    /// and `var_names` as declaredVarNames. A step per declaration and per check.
    fn global_declaration_instantiation(
        &mut self,
        compiled: &Rc<Compiled>,
        script: Handle,
        realm: Handle,
    ) -> Completion<()> {
        let code = &compiled.code;
        // Step 8: the functions, in reverse List order.
        for &function in code.function_declarations.iter().rev() {
            self.step()?;
            let name = self.name_key_of(compiled, function)?;
            if !self.can_declare_global_function(realm, name)? {
                return Err(self.throw_error(
                    ErrorKind::TypeError,
                    "a global function cannot be declared over this property",
                ));
            }
        }
        // Step 10: the variables.
        for &variable in &code.var_names {
            self.step()?;
            let name = self.string_key_of(compiled, variable)?;
            if !self.can_declare_global_var(realm, name)? {
                return Err(self.throw_error(
                    ErrorKind::TypeError,
                    "a global variable cannot be declared on this global object",
                ));
            }
        }
        // Step 17.
        for &function in &code.function_declarations {
            self.step()?;
            let name = self.name_key_of(compiled, function)?;
            let made = self.instantiate_function(compiled, script, realm, function, None)?;
            let keep = self.keep(Value::Object(made));
            let bound =
                self.create_global_function_binding(realm, name, Value::Object(made), false);
            self.release(keep);
            bound?;
        }
        // Step 18.
        for &variable in &code.var_names {
            self.step()?;
            let name = self.string_key_of(compiled, variable)?;
            self.create_global_var_binding(realm, name, false)?;
        }
        Ok(())
    }

    /// The property key of the name of function template `index`.
    pub(crate) fn name_key_of(&mut self, compiled: &Compiled, index: u32) -> Completion<Key> {
        match compiled
            .name_keys
            .get(usize::try_from(index).unwrap_or(usize::MAX))
        {
            Some(&key) => Ok(key),
            None => Err(self.fault()),
        }
    }

    /// The property key of String constant `index`.
    pub(crate) fn string_key_of(&mut self, compiled: &Compiled, index: u32) -> Completion<Key> {
        match compiled
            .keys
            .get(usize::try_from(index).unwrap_or(usize::MAX))
        {
            Some(&key) => Ok(key),
            None => Err(self.fault()),
        }
    }

    /// [[HostDefined]] of the script record `script`.
    pub(crate) fn script_record_host_defined(&self, script: Handle) -> Option<HostDefined> {
        self.heap.scripts.get(script).ok()?.host_defined()
    }
}

/// The value of an abrupt completion that ended a parse: a throw cannot happen, since parsing
/// runs no code.
pub(crate) fn parse_termination(abrupt: Abrupt) -> Termination {
    match abrupt {
        Abrupt::Terminated(termination) => termination,
        Abrupt::Throw(_) => Termination::EngineFault,
    }
}
