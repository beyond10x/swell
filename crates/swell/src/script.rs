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
//! the call in `Terminated(EngineFault)`. ParseScript runs no code, calls no hook and spends no
//! step: `parse_script` and `cx.parse_script` read the interrupt flag before they call it
//! (HOST-166, HOST-167), and its parse and compile step read it through their ledger.
//!
//! The parse and the compile step run on the agent's ledger ([`AgentLedger`]): the tree, the
//! scope analysis, the compile step's scratch and the code are charged to the memory budget
//! before they grow and credited when they drop; a poll at least once per 4,096 code units or
//! nodes reads the interrupt flag and clears it. A refusal ends the run in
//! `Terminated(MemoryBudget)`, a read flag in `Terminated(Interrupted)`, with everything the step
//! charged credited. The tree's charge ([`swell_syntax::Script::charged`]) is credited when the
//! tree is dropped, after the compile step; the code's ([`Code::charged`]) passes to the record.
//!
//! The record owns the compiled code and the atoms of its String constants, of their property
//! keys and of its functions' names; the record and its atom vectors are charged before they are
//! allocated, the atoms as they are interned, and the sweep that frees the record credits them
//! with the code.
//!
//! Source text (`docs/design/engine.md`, decision 4, "Source text."): the copy the parse read is
//! kept with the compiled code ([`Compiled::source`]) and its charge becomes part of the
//! record's when the parse and compile step ends, two bytes per code unit, with no credit
//! between ([`Agent::make_script_record`]); the sweep that frees the record credits it with the
//! code, never earlier. The record lives while
//! the host holds the script, while its code runs and while a function made from it is
//! reachable (each reports its [[ScriptOrModule]]), so the text does too, and
//! `Function.prototype.toString` reads a function's [[SourceText]] out of it
//! (`builtins/function.rs`). A parse or compile step that makes no code credits the copy
//! ([`Agent::credit_source_copy`]). Eval code and the `Function` constructor keep theirs the same
//! way (`dynamic_code.rs`).
//!
//! Kind site of the collector: [`ScriptRecord::trace`] (the realm, the constants' atoms and the
//! template objects).

use alloc::boxed::Box;
use alloc::format;
use alloc::rc::Rc;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;

use swell_compiler::CompileError;
use swell_compiler::bytecode::Code;
use swell_syntax::Failure;
use swell_syntax::ParseOptions;
use swell_syntax::ledger::{Ledger, Stop};

use crate::heap::accounting::Accounting;
use crate::heap::arena::Handle;
use crate::heap::collect::{Item, Mark};
use crate::host::agent::Agent;
use crate::host::host_object::{Brand, HostData, HostObject, Tracer};
use crate::host::outcome::{Abrupt, Completion, Termination};
use crate::host::{ErrorKind, Host, HostDefined, InterruptHandle, Source};
use crate::interp::frame::{Frame, SlotValue, frame_bytes};
use crate::object::property::{Key, array_index};
use crate::value::{ObjectRef, StringRef, Value};

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
    /// The code is eval code or the code of a `Function` constructor, its positions counted in
    /// that code's own text (decision 13, `ErrorSite::dynamic`).
    pub(crate) dynamic: bool,
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
    /// The realm's [[TemplateMap]] for the code of the record (`#sec-gettemplateobject`;
    /// `docs/design/engine.md`, decision 4, "Template objects", item 2): the template object of
    /// each tagged template site of the code by its number, `None` until the site's first
    /// evaluation (`interp/templates.rs`); charged with the record, freed with it.
    pub(crate) templates: Vec<Option<ObjectRef>>,
}

impl ScriptRecord {
    /// Kind site of the collector: [[Realm]], the atoms of the code's constants and of its
    /// functions' names (the property keys are the same atoms), [[HostDefined]] and the
    /// template objects of the sites evaluated.
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
        for &template in self.templates.iter().flatten() {
            mark.object(template);
        }
    }

    /// The host-defined value.
    pub(crate) fn host_defined(&self) -> Option<HostDefined> {
        let data: &dyn core::any::Any = self.host_defined.as_ref()?.data.as_deref()?;
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
            templates: Vec::new(),
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

/// What a charged parse answers inside the engine when it was not stopped. The tree is not boxed:
/// a box would be an allocation the budget does not see, for a value that lives one call.
#[expect(
    clippy::large_enum_variant,
    reason = "a box would be an allocation the budget does not see, for a value that lives one call"
)]
pub(crate) enum ParsedTree {
    /// The tree, whose parse charged [`swell_syntax::Script::charged`], with the copy of its
    /// source.
    Tree(swell_syntax::Script, SourceCopy),
    /// The parse errors, at least one; the copy was credited.
    Errors(Vec<swell_syntax::SyntaxError>),
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

/// The ledger of a parse or compile step (decision 4, "Memory budget"): a charge goes to the
/// memory budget and a refusal is `Stop::OverBudget`; a credit gives bytes back; a poll reads the
/// interrupt flag and clears it as [`Agent::check_interrupt`] does, `Stop::Interrupted` when it
/// was set. Under test, the countdown of [`Accounting::set_poll_countdown`] sets the flag at the
/// poll it names, before the poll reads it.
pub(crate) struct AgentLedger<'a> {
    accounting: &'a mut Accounting,
    interrupt: &'a InterruptHandle,
}

impl Ledger for AgentLedger<'_> {
    fn charge(&mut self, bytes: usize) -> Result<(), Stop> {
        let bytes = u64::try_from(bytes).unwrap_or(u64::MAX);
        self.accounting.charge(bytes).map_err(|_| Stop::OverBudget)
    }

    fn credit(&mut self, bytes: usize) {
        self.accounting
            .credit(u64::try_from(bytes).unwrap_or(u64::MAX));
    }

    fn poll(&mut self) -> Result<(), Stop> {
        #[cfg(test)]
        if self.accounting.count_down_poll() {
            self.interrupt.interrupt();
        }
        if self.interrupt.is_set() {
            self.interrupt.clear();
            return Err(Stop::Interrupted);
        }
        Ok(())
    }
}

/// The bytes of the `Rc` allocation of a [`Compiled`]: its two counts and the value.
const COMPILED_BOX_BYTES: usize = size_of::<Compiled>() + 2 * size_of::<usize>();

/// What a script record owns besides its code and its source copy, charged before it is
/// allocated: the `Rc` allocation of its [`Compiled`], the four atom vectors of `strings` String
/// constants and `functions` templates, and the box of its [[HostDefined]] when it has one.
fn record_bytes(strings: usize, functions: usize, host_defined: bool) -> u64 {
    let host = if host_defined {
        size_of::<ScriptHostDefined>()
    } else {
        0
    };
    u64::try_from(COMPILED_BOX_BYTES.saturating_add(host))
        .unwrap_or(u64::MAX)
        .saturating_add(vec_bytes::<StringRef>(strings))
        .saturating_add(vec_bytes::<Key>(strings))
        .saturating_add(vec_bytes::<StringRef>(functions))
        .saturating_add(vec_bytes::<Key>(functions))
}

/// A vector with room for exactly `n` elements, whose bytes are charged already; `None` when the
/// allocator refuses.
fn exact<T>(n: usize) -> Option<Vec<T>> {
    let mut v = Vec::new();
    #[expect(
        clippy::disallowed_methods,
        reason = "charged by `make_script_record` with the record before it is reserved (`record_bytes`, the template sites)"
    )]
    v.try_reserve_exact(n).ok()?;
    Some(v)
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
        let parsed = swell_syntax::parse_script_with(&copy.units, options, &mut self.ledger());
        let (script, copy) = match self.parsed_tree(parsed, copy)? {
            ParsedTree::Tree(script, copy) => (script, copy),
            ParsedTree::Errors(errors) => {
                return self.syntax_errors(realm, errors).map(ParseAnswer::Errors);
            }
        };
        let compiled = swell_compiler::compile_script_with(&script, &mut self.ledger());
        self.drop_tree(script);
        let (code, copy) = match self.compiled_with_source(realm, compiled, copy)? {
            Ok(made) => made,
            Err(error) => return Ok(ParseAnswer::Errors(vec![error])),
        };
        self.make_script_record(realm, code, copy, host_defined, false)
            .map(ParseAnswer::Script)
    }

    /// The ledger of a parse or compile step of this agent.
    pub(crate) fn ledger(&mut self) -> AgentLedger<'_> {
        AgentLedger {
            accounting: &mut self.heap.accounting,
            interrupt: &self.interrupt,
        }
    }

    /// The answer of a charged parse of the source whose copy is `copy`: the tree with the copy,
    /// or the errors with the copy credited; a stopped parse (which credited what it charged)
    /// ends the run, the copy credited.
    pub(crate) fn parsed_tree(
        &mut self,
        parsed: Result<swell_syntax::Script, Failure>,
        copy: SourceCopy,
    ) -> Completion<ParsedTree> {
        match parsed {
            Ok(script) => Ok(ParsedTree::Tree(script, copy)),
            Err(Failure::Errors(errors)) => {
                self.credit_source_copy(copy);
                Ok(ParsedTree::Errors(errors))
            }
            Err(Failure::Stopped(stop)) => {
                self.credit_source_copy(copy);
                Err(self.stopped(stop))
            }
        }
    }

    /// Drop the tree `script` and credit what its parse charged for it.
    pub(crate) fn drop_tree(&mut self, script: swell_syntax::Script) {
        let charged = script.charged;
        drop(script);
        self.heap
            .accounting
            .credit(u64::try_from(charged).unwrap_or(u64::MAX));
    }

    /// Drop `code` and credit what its compile step charged for it.
    pub(crate) fn drop_code(&mut self, code: Code) {
        let charged = code.charged;
        drop(code);
        self.heap
            .accounting
            .credit(u64::try_from(charged).unwrap_or(u64::MAX));
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
        #[expect(
            clippy::disallowed_methods,
            reason = "charged before it is reserved, `vec_bytes::<u16>` of the copy"
        )]
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
    /// line and column; each refusal among them is counted ([`Agent::refusals`]), and the line
    /// and column of the first are kept for `parse_script`'s error site (decision 13). Every
    /// parse of the engine answers its errors here: `parse_script`, `eval` and `Function`.
    pub(crate) fn syntax_errors(
        &mut self,
        realm: Handle,
        errors: Vec<swell_syntax::SyntaxError>,
    ) -> Completion<Vec<Value>> {
        let mut objects = Vec::new();
        for error in errors {
            if objects.is_empty() {
                self.syntax_error_at = Some((error.line, error.column));
            }
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
            Err(error @ (CompileError::TooMany(_) | CompileError::TooManyInClass(_))) => {
                let message = error.to_string();
                let object = self.new_error_in(realm, ErrorKind::SyntaxError, &message)?;
                Ok(Err(Value::Object(object)))
            }
            Err(CompileError::Tree(_)) => Err(self.fault()),
            Err(CompileError::Stopped(stop)) => Err(self.stopped(stop)),
        }
    }

    /// The end of a parse or compile step its ledger stopped: over the budget the run ends in
    /// `Terminated(MemoryBudget)`, at an interrupt in `Terminated(Interrupted)` (the ledger
    /// cleared the flag as it read it).
    pub(crate) fn stopped(&mut self, stop: Stop) -> Abrupt {
        match stop {
            Stop::OverBudget => self.terminate(Termination::MemoryBudget),
            Stop::Interrupted => self.terminate(Termination::Interrupted),
        }
    }

    /// The Script Record of `code` in `realm`, with its atoms interned and the source `copy` it
    /// was compiled from kept with it: the code's charge ([`Code::charged`], taken from its
    /// compile step) and the copy's become the record's, with no credit and no check between,
    /// and the sweep that frees the record credits them (decision 4, "Source text."). The record
    /// and its atom vectors are charged before they are allocated; on every way out without a
    /// record everything is credited. `dynamic` is true for eval code and the code of a
    /// `Function` constructor ([`Compiled::dynamic`]).
    pub(crate) fn make_script_record(
        &mut self,
        realm: Handle,
        code: Code,
        copy: SourceCopy,
        host_defined: Option<HostDefined>,
        dynamic: bool,
    ) -> Completion<Handle> {
        let SourceCopy {
            units: source,
            charged: copy_charged,
        } = copy;
        let taken = u64::try_from(code.charged)
            .unwrap_or(u64::MAX)
            .saturating_add(copy_charged);
        let own = record_bytes(
            code.strings.len(),
            code.functions.len(),
            host_defined.is_some(),
        )
        .saturating_add(vec_bytes::<Option<ObjectRef>>(code.template_sites.len()));
        if self.heap.accounting.charge(own).is_err() {
            drop((code, source));
            self.heap.accounting.credit(taken);
            return Err(self.terminate(Termination::MemoryBudget));
        }
        let owned = taken.saturating_add(own);
        let (strings, keys, names, name_keys) = match self.record_atoms(&code) {
            Ok(atoms) => atoms,
            Err(abrupt) => {
                drop((code, source));
                self.heap.accounting.credit(owned);
                return Err(abrupt);
            }
        };
        let Some(mut templates) = exact(code.template_sites.len()) else {
            drop((code, source));
            self.heap.accounting.credit(owned);
            return Err(self.terminate(Termination::MemoryBudget));
        };
        templates.resize(code.template_sites.len(), None);
        let record = ScriptRecord {
            realm: Some(realm),
            compiled: Some(Rc::new(Compiled {
                code,
                source,
                strings,
                keys,
                names,
                name_keys,
                dynamic,
            })),
            host_defined: host_defined.map(|value| HostObject {
                brand: Brand(0),
                data: Some(Box::new(ScriptHostDefined(value))),
                error_kind: None,
            }),
            templates,
        };
        match crate::heap::store_charged(
            &mut self.heap.accounting,
            &mut self.heap.scripts,
            record,
            owned,
        ) {
            Ok(handle) => Ok(handle),
            Err(_) => {
                self.heap.accounting.credit(owned);
                Err(self.terminate(Termination::MemoryBudget))
            }
        }
    }

    /// The atoms of the String constants of `code` and of its templates' names, with their
    /// property keys, in vectors whose bytes [`record_bytes`] charged. The atoms are reported by
    /// no site until the record holds them; no safe point comes before it does.
    #[expect(
        clippy::type_complexity,
        reason = "the four vectors of one record, answered together and taken apart by the one caller"
    )]
    fn record_atoms(
        &mut self,
        code: &Code,
    ) -> Completion<(Vec<StringRef>, Vec<Key>, Vec<StringRef>, Vec<Key>)> {
        let (Some(mut strings), Some(mut keys), Some(mut names), Some(mut name_keys)) = (
            exact(code.strings.len()),
            exact(code.strings.len()),
            exact(code.functions.len()),
            exact(code.functions.len()),
        ) else {
            return Err(self.terminate(Termination::MemoryBudget));
        };
        for units in &code.strings {
            let (atom, key) = self.atom_and_key(units)?;
            strings.push(atom);
            keys.push(key);
        }
        for function in &code.functions {
            let (atom, key) = self.atom_and_key(&function.name)?;
            names.push(atom);
            name_keys.push(key);
        }
        Ok((strings, keys, names, name_keys))
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
        let charged = frame_bytes().saturating_add(vec_bytes::<SlotValue>(count));
        if self.heap.accounting.charge(charged).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        let mut locals = Vec::new();
        #[expect(
            clippy::disallowed_methods,
            reason = "charged with the frame before it is reserved, `vec_bytes::<SlotValue>` of the locals"
        )]
        if locals.try_reserve_exact(count).is_err() {
            self.heap.accounting.credit(charged);
            return Err(self.terminate(Termination::MemoryBudget));
        }
        locals.resize(count, SlotValue::Value(Value::Undefined));
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
            new_target: None,
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

    /// GlobalDeclarationInstantiation (`#sec-globaldeclarationinstantiation`) of a script, with
    /// the compiler's `lexical` as its LexicallyDeclaredNames (`let`, `const` and class),
    /// `function_declarations` as functionsToInitialize and `var_names` as declaredVarNames. A
    /// step per declaration and per check (decision 3, "Lexical bindings", item 6). Steps 3 and
    /// 4 throw before anything is bound, so a script whose names clash with an earlier one's
    /// leaves the global environment as it was.
    fn global_declaration_instantiation(
        &mut self,
        compiled: &Rc<Compiled>,
        script: Handle,
        realm: Handle,
    ) -> Completion<()> {
        let code = &compiled.code;
        // Step 3: no lexical name may be bound by an earlier script's lexical declaration, nor
        // be an own non-configurable property of the global object (a `var` or function of an
        // earlier script, or one such as `undefined`).
        for lexical in &code.lexical {
            self.step()?;
            let name = self.string_key_of(compiled, lexical.name)?;
            if self.has_lexical_declaration(realm, name)?
                || self.has_restricted_global_property(realm, name)?
            {
                return Err(self.already_declared(name));
            }
        }
        // Step 4: no `var` or function name may be bound by a lexical declaration.
        for &function in &code.function_declarations {
            self.step()?;
            let name = self.name_key_of(compiled, function)?;
            if self.has_lexical_declaration(realm, name)? {
                return Err(self.already_declared(name));
            }
        }
        for &variable in &code.var_names {
            self.step()?;
            let name = self.string_key_of(compiled, variable)?;
            if self.has_lexical_declaration(realm, name)? {
                return Err(self.already_declared(name));
            }
        }
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
        // Step 15: the lexical bindings, uninitialized until their declarations run.
        for lexical in &code.lexical {
            self.step()?;
            let name = self.string_key_of(compiled, lexical.name)?;
            self.create_global_lexical(realm, name, lexical.constant)?;
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

    /// The `SyntaxError` of GlobalDeclarationInstantiation, steps 3 and 4, and of
    /// EvalDeclarationInstantiation, step 3: `name` is already declared where it would be
    /// declared again.
    pub(crate) fn already_declared(&mut self, name: Key) -> Abrupt {
        let text = self.key_text(name);
        let message = format!("{text} has already been declared");
        self.throw_error(ErrorKind::SyntaxError, &message)
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

#[cfg(test)]
mod tests {
    use alloc::format;
    use alloc::string::String;
    use alloc::vec::Vec;
    use std::println;
    use swell_syntax::ledger::{Ledger, Stop};
    use swell_syntax::{ParseOptions, Script as Tree};

    use crate::host::agent::Agent;
    use crate::host::hooks::test_host::{NoHooks, agent_with, limits, new_realm};
    use crate::{Outcome, Parsed, PropertyKey, Realm, Script, Source, Termination, Value};

    const MIB: u64 = 1 << 20;
    const OPTIONS: ParseOptions = ParseOptions { max_nesting: 100 };

    /// The lines of the flat source: 10,000 in the default mode's lane `slow`
    /// (`SWELL_FULL_COUNTS=1`), 5,000 in the gate and with the feature
    /// `collect-every-safe-point`. Each test of the flat source compares readings of the same
    /// source with each other (a peak with its floor, a budget below a peak), so the outcome
    /// each asserts holds at either count, and 5,000 lines are 35,000 code units, 8 polls of the
    /// parse. With the story's 20,000 the four tests took 6.5 CPU seconds together in either
    /// collection mode, each run alone, and HOST-169's 3.1 s of them was the slowest test of a
    /// whole-package run in both modes (21 s at a load of 60 to 95 on 20 cores; 2.9 and 3.1 CPU
    /// seconds alone in the lane's two modes, measured on 2026-11-22).
    fn flat_lines() -> usize {
        // HOST-2 (source_scan.rs): no run-time environment read in engine sources, tests included.
        if matches!(option_env!("SWELL_FULL_COUNTS"), Some("1"))
            && !cfg!(feature = "collect-every-safe-point")
        {
            10_000
        } else {
            5_000
        }
    }

    /// `function unused() {`, [`flat_lines`] lines `x = 1;`, then `}`.
    fn flat_source() -> String {
        let mut text = String::from("function unused() {\n");
        text.push_str(&"x = 1;\n".repeat(flat_lines()));
        text.push('}');
        text
    }

    /// The body of the `Function` call: the [`flat_lines`] lines alone.
    fn flat_body() -> String {
        "x = 1;\n".repeat(flat_lines())
    }

    fn units(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    /// The text of the last parse of `Function(body)`.
    fn dynamic_text(body: &str) -> String {
        format!("function anonymous(\n) {{\n{body}\n}}")
    }

    fn bytes(count: usize) -> u64 {
        u64::try_from(count).unwrap_or(u64::MAX)
    }

    /// An agent whose budget is 64 MiB, with a realm.
    fn agent() -> (Agent<NoHooks>, Realm) {
        let mut limits = limits();
        limits.memory_bytes = 64 * MIB;
        limits.steps_per_entry = 10_000_000;
        let mut agent = agent_with(limits);
        let realm = new_realm(&mut agent);
        (agent, realm)
    }

    fn parse(agent: &mut Agent<NoHooks>, realm: &Realm, text: &str, utf16: bool) -> Parsed<Script> {
        let wide = units(text);
        let source = if utf16 {
            Source::Utf16(&wide)
        } else {
            Source::Utf8(text)
        };
        match agent.parse_script(realm, source, None) {
            Ok(parsed) => parsed,
            Err(error) => panic!("parse_script answered {error:?}"),
        }
    }

    fn parsed(agent: &mut Agent<NoHooks>, realm: &Realm, text: &str) -> Script {
        match parse(agent, realm, text, false) {
            Parsed::Parsed(script) => script,
            Parsed::Errors(_) => panic!("{text:?} did not parse"),
            Parsed::Terminated(termination) => panic!("{text:?} ended in {termination:?}"),
        }
    }

    fn heap_bytes(agent: &Agent<NoHooks>) -> u64 {
        match agent.heap_bytes() {
            Ok(bytes) => bytes,
            Err(error) => panic!("heap_bytes answered {error:?}"),
        }
    }

    fn collect(agent: &mut Agent<NoHooks>) {
        if let Err(error) = agent.collect() {
            panic!("collect answered {error:?}");
        }
    }

    /// A free slot in the scripts arena: a script parsed, dropped and collected.
    fn free_slot(agent: &mut Agent<NoHooks>, realm: &Realm) {
        drop(parsed(agent, realm, "1"));
        collect(agent);
    }

    /// The bytes the scripts arena's slots own: what the record in a freed slot owns is the
    /// difference of this reading.
    fn scripts_owned(agent: &Agent<NoHooks>) -> u64 {
        agent.heap.scripts.ledger_floor()
    }

    /// The tree walk (`Script::capacity_bytes`) of `text` as a script.
    fn tree_of_script(text: &str) -> u64 {
        match swell_syntax::parse_script(&units(text), OPTIONS) {
            Ok(tree) => bytes(tree.capacity_bytes()),
            Err(errors) => panic!("{errors:?}"),
        }
    }

    /// The tree walk of the last parse of `Function(body)`.
    fn tree_of_function(body: &str) -> u64 {
        match swell_syntax::parse_dynamic_function(&[], &units(body), OPTIONS) {
            Ok(tree) => bytes(tree.capacity_bytes()),
            Err(errors) => panic!("{errors:?}"),
        }
    }

    /// A ledger with no bound that reads its high-water mark.
    #[derive(Default)]
    struct Peak {
        held: usize,
        high: usize,
    }

    impl Ledger for Peak {
        fn charge(&mut self, bytes: usize) -> Result<(), Stop> {
            self.held = self.held.saturating_add(bytes);
            self.high = self.high.max(self.held);
            Ok(())
        }
        fn credit(&mut self, bytes: usize) {
            self.held = self.held.saturating_sub(bytes);
        }
        fn poll(&mut self) -> Result<(), Stop> {
            Ok(())
        }
    }

    /// The high-water mark of the parse alone (the tree with its scratch, before the compile
    /// step), measured through the charged entry.
    fn parse_peak(text: &str) -> u64 {
        let mut peak = Peak::default();
        let tree: Result<Tree, _> =
            swell_syntax::parse_script_with(&units(text), OPTIONS, &mut peak);
        assert!(tree.is_ok());
        drop(tree);
        bytes(peak.high)
    }

    /// Define the global Strings `src` (the flat source) and `body` (its lines alone).
    fn define_sources(agent: &mut Agent<NoHooks>, realm: &Realm) {
        let global = match agent.global_object(realm) {
            Ok(root) => root,
            Err(error) => panic!("global_object answered {error:?}"),
        };
        let defined = agent.with_cx(realm, |cx| {
            let Ok(Value::Object(object)) = cx.get(&global) else {
                panic!("the global is no object");
            };
            for (name, text) in [("src", flat_source()), ("body", flat_body())] {
                let (Ok(key), Ok(value)) = (cx.new_string(name), cx.new_string(&text)) else {
                    panic!("new_string failed");
                };
                assert!(matches!(
                    cx.set(object, PropertyKey::String(key), Value::String(value), true),
                    Ok(Ok(true))
                ));
            }
        });
        assert!(defined.is_ok());
    }

    fn evaluate(agent: &mut Agent<NoHooks>, script: &Script) -> Outcome<crate::Root> {
        match agent.evaluate_script(script) {
            Ok(outcome) => outcome,
            Err(error) => panic!("evaluate_script answered {error:?}"),
        }
    }

    /// HOST-169: a flat source ([`flat_source`]), as UTF-8 and as UTF-16, with a free slot in
    /// the scripts arena. P is the high-water mark of a successful parse over what the heap
    /// read before, T the tree walk, R what the record owns. Under P the parse succeeds; under
    /// P - T, and under R + 4 KiB, the tree does not fit and the parse ends in
    /// `Terminated(MemoryBudget)`; after each refusal the heap reads as before and the next
    /// parse succeeds.
    #[test]
    fn host_169_a_flat_source_whose_tree_does_not_fit_ends_in_memory_budget() {
        let text = flat_source();
        let t = tree_of_script(&text);
        for utf16 in [false, true] {
            let (mut agent, realm) = agent();
            // The atoms of the source stay live, so its record is all a parse leaves.
            let _kept = parsed(&mut agent, &realm, &text);
            free_slot(&mut agent, &realm);
            let before = heap_bytes(&agent);
            let owned_before = scripts_owned(&agent);
            agent.heap.accounting.reset_high_water();
            let Parsed::Parsed(script) = parse(&mut agent, &realm, &text, utf16) else {
                panic!("the flat source did not parse");
            };
            let p = agent.heap.accounting.high_water() - before;
            let r = scripts_owned(&agent) - owned_before;
            assert_eq!(
                heap_bytes(&agent),
                before + r,
                "the record is all that stays"
            );
            drop(script);
            collect(&mut agent);
            println!("utf16 {utf16}: P {p} T {t} R {r}");
            for (amount, expected) in [
                (p, None),
                (p - t, Some(Termination::MemoryBudget)),
                (r + 4096, Some(Termination::MemoryBudget)),
            ] {
                let before = heap_bytes(&agent);
                agent.heap.accounting.set_limit(before + amount);
                match (parse(&mut agent, &realm, &text, utf16), expected) {
                    (Parsed::Parsed(script), None) => {
                        drop(script);
                        agent.heap.accounting.set_limit(64 * MIB);
                        collect(&mut agent);
                    }
                    (Parsed::Terminated(termination), Some(expected)) => {
                        assert_eq!(termination, expected, "budget before + {amount}");
                        assert_eq!(heap_bytes(&agent), before, "budget before + {amount}");
                        let Parsed::Parsed(one) = parse(&mut agent, &realm, "1", utf16) else {
                            panic!("1 did not parse after the refusal");
                        };
                        drop(one);
                        agent.heap.accounting.set_limit(64 * MIB);
                        collect(&mut agent);
                    }
                    (_, expected) => panic!("budget before + {amount}: expected {expected:?}"),
                }
            }
        }
    }

    /// The peak of a parse covers the copy of its source and the tree: for `parse_script` as
    /// UTF-8 and as UTF-16, for `eval(src)` and for `Function(body)`.
    #[test]
    fn the_peak_of_a_parse_covers_copy_tree_and_code() {
        let text = flat_source();
        let body = flat_body();
        let script_floor = bytes(units(&text).len() * 2) + tree_of_script(&text);
        for utf16 in [false, true] {
            let (mut agent, realm) = agent();
            let before = heap_bytes(&agent);
            agent.heap.accounting.reset_high_water();
            assert!(matches!(
                parse(&mut agent, &realm, &text, utf16),
                Parsed::Parsed(_)
            ));
            let peak = agent.heap.accounting.high_water() - before;
            assert!(
                peak >= script_floor,
                "utf16 {utf16}: {peak} < {script_floor}"
            );
        }
        let function_floor = bytes(units(&dynamic_text(&body)).len() * 2) + tree_of_function(&body);
        for (call, floor) in [
            ("eval(src);", script_floor),
            ("Function(body);", function_floor),
        ] {
            let (mut agent, realm) = agent();
            define_sources(&mut agent, &realm);
            let script = parsed(&mut agent, &realm, call);
            collect(&mut agent);
            let before = heap_bytes(&agent);
            agent.heap.accounting.reset_high_water();
            assert!(matches!(evaluate(&mut agent, &script), Outcome::Normal(_)));
            let peak = agent.heap.accounting.high_water() - before;
            assert!(peak >= floor, "{call}: {peak} < {floor}");
        }
    }

    /// With a free slot in the scripts arena and the source's atoms live, a successful parse
    /// leaves its record charged, the bytes of the slot; a parse stopped at the copy, in the
    /// tree or in the compile step leaves the heap as it read before.
    #[test]
    fn a_parse_leaves_only_its_record_charged() {
        let text = flat_source();
        let copy = bytes(units(&text).len() * 2);
        let parse_peak = parse_peak(&text);
        let (mut agent, realm) = agent();
        let _kept = parsed(&mut agent, &realm, &text);
        free_slot(&mut agent, &realm);
        let before = heap_bytes(&agent);
        let owned_before = scripts_owned(&agent);
        agent.heap.accounting.reset_high_water();
        let script = parsed(&mut agent, &realm, &text);
        let p = agent.heap.accounting.high_water() - before;
        let slot = scripts_owned(&agent) - owned_before;
        assert_eq!(heap_bytes(&agent), before + slot);
        drop(script);
        collect(&mut agent);
        // The copy, the tree and the compile step's scratch with the code.
        let at_copy = copy / 2;
        let in_tree = copy + parse_peak / 2;
        let in_compile = copy + parse_peak + (p - copy - parse_peak) / 2;
        println!("copy {copy} parse peak {parse_peak} P {p} slot {slot}");
        for amount in [at_copy, in_tree, in_compile] {
            let before = heap_bytes(&agent);
            agent.heap.accounting.set_limit(before + amount);
            assert!(
                matches!(
                    parse(&mut agent, &realm, &text, false),
                    Parsed::Terminated(Termination::MemoryBudget)
                ),
                "budget before + {amount}"
            );
            assert_eq!(heap_bytes(&agent), before, "budget before + {amount}");
            agent.heap.accounting.set_limit(64 * MIB);
            collect(&mut agent);
        }
    }

    /// The countdown of polls at 2: at the second poll the ledger sets the interrupt flag, and
    /// `parse_script` (as UTF-8 and as UTF-16), `eval(src)` and `Function(body)` each end in
    /// `Terminated(Interrupted)`, with the countdown at 0 and the heap as it read before.
    #[test]
    fn a_flag_raised_at_the_second_poll_ends_every_entry() {
        let text = flat_source();
        for utf16 in [false, true] {
            let (mut agent, realm) = agent();
            collect(&mut agent);
            let before = heap_bytes(&agent);
            agent.heap.accounting.set_poll_countdown(2);
            assert!(matches!(
                parse(&mut agent, &realm, &text, utf16),
                Parsed::Terminated(Termination::Interrupted)
            ));
            assert_eq!(agent.heap.accounting.poll_countdown(), Some(0));
            assert_eq!(heap_bytes(&agent), before);
        }
        for call in ["eval(src);", "Function(body);"] {
            let (mut agent, realm) = agent();
            define_sources(&mut agent, &realm);
            let script = parsed(&mut agent, &realm, call);
            collect(&mut agent);
            let before = heap_bytes(&agent);
            agent.heap.accounting.set_poll_countdown(2);
            assert!(
                matches!(
                    evaluate(&mut agent, &script),
                    Outcome::Terminated(Termination::Interrupted)
                ),
                "{call}"
            );
            assert_eq!(agent.heap.accounting.poll_countdown(), Some(0), "{call}");
            // The String sourceString that `Function` builds before its parse (steps 12 to 15)
            // is a heap String no root holds once the run ends: a collection frees it.
            collect(&mut agent);
            assert_eq!(heap_bytes(&agent), before, "{call}");
        }
    }
}
