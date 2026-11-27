//! Source text compiled at run time: `eval` (`#sec-eval-x`, PerformEval `#sec-performeval`,
//! EvalDeclarationInstantiation `#sec-evaldeclarationinstantiation`, and the direct call of
//! `#sec-function-calls-runtime-semantics-evaluation`, step 6) and the `Function` constructor
//! (`#sec-function-p1-p2-pn-body`, CreateDynamicFunction `#sec-createdynamicfunction`, kind
//! `normal`). Written by `story:eval-and-function-constructor`. CreateDynamicFunction of kind
//! `generator`, for `%GeneratorFunction%` (`#sec-generatorfunction`, `builtins/generator.rs`),
//! is `story:generator-function-constructor`: the same steps, with the prefix `function*`, the
//! fallback prototype `%GeneratorFunction.prototype%`, the parse entry of that kind and the
//! same hook, whose compilation type is `Function` for every kind (step 16). CreateDynamicFunction
//! of kind `async`, for `%AsyncFunction%` (`#sec-async-function-constructor-arguments`,
//! `builtins/async_function.rs`), is `story:async-function-constructor`, the same way: the
//! prefix `async function`, the fallback prototype `%AsyncFunction.prototype%` and the parse
//! entry of that kind. CreateDynamicFunction of kind `async-generator`, for
//! `%AsyncGeneratorFunction%` (`#sec-asyncgeneratorfunction`, `builtins/async_generator.rs`), is
//! `story:async-generator-function-constructor`, the same way: the prefix `async function*`, the
//! fallback prototype `%AsyncGeneratorFunction.prototype%` and the parse entry of that kind.
//!
//! Both ask the host first, HostEnsureCanCompileStrings (`#sec-hostensurecancompilestrings`,
//! with its seven parameters) before anything is parsed; `eval` of an object and each argument
//! of `Function` that is an object ask HostGetCodeForEval (`#sec-hostgetcodeforeval`) for its
//! source text. Every answer of a hook that holds a handle is checked before it is used, as the
//! answer of a host function is (`docs/design/host-interface.md`, decision 6): a stale handle
//! or a handle of another agent ends the run in `Terminated(EngineFault)`. A hook called while no
//! builtin runs (the direct `eval`) runs under an activation of `%eval%`, so that code a hook
//! runs through its `Cx` is a re-entry counted as from a builtin, and spends the steps of the
//! running entry.
//!
//! Both parse through one charged entry of `swell-syntax` (`parse_eval_with` or
//! `parse_dynamic_function_with`) on the agent's ledger, with the agent's `parser_nesting`, so
//! the nesting limit is the `SyntaxError` naming `parser_nesting` (decision 9, item 9); a parse
//! error is a `SyntaxError` of the current realm, counted when it is a refusal (`script.rs`,
//! [`Agent::refusals`]). Both read the interrupt flag before they copy the source; `eval` charges
//! a step per code unit of its String before that. The UTF-16 copy of the source is charged to
//! the memory budget from the start of the parse, the tree and the compile step's scratch and
//! code as they grow, and the record and its atoms when it is made, as `parse_script` charges
//! them (`script.rs`).
//! The code is kept in a script record of its own, and the copy with it (`docs/design/engine.md`,
//! decision 4, "Source text."): for eval code the String PerformEval parsed, for the `Function`
//! constructor the whole text of its last parse, `function anonymous(` P `\n) {\n` body `\n}`
//! (`#sec-createdynamicfunction`; `function* anonymous(` for kind `generator`, `async function
//! anonymous(` for kind `async`, `async function* anonymous(` for kind `async-generator`), of
//! which the function's [[SourceText]] is all.
//!
//! Eval code runs in a frame of its own: the direct `eval` pushes it on the running loop, as a
//! call of an ECMAScript function is (`interp/dispatch.rs`), with no Rust recursion; the
//! indirect `eval` is a builtin, whose code is a re-entry (`interp/mod.rs`).

use alloc::rc::Rc;
use alloc::vec;
use alloc::vec::Vec;

use swell_compiler::bytecode::Slot;
use swell_syntax::{EvalContext, ParseOptions};

use crate::builtins::object::argument;
use crate::builtins::string::StringBuilder;
use crate::env::{Bound, NameTable, RecordNames, SlotKind, SlotNames};
use crate::heap::accounting::ChargedVec;
use crate::heap::arena::Handle;
use crate::heap::string::CodeUnits;
use crate::host::agent::Agent;
use crate::host::outcome::{Abrupt, Completion, Termination};
use crate::host::{CompilationType, CompileStrings, Cx, HookCx, Host};
use crate::interp::frame::{Constructed, Frame, SlotValue, frame_bytes};
use crate::object::ObjectKind;
use crate::object::builtin_function::{Activation, KeptRoom};
use crate::object::class_constructor::ConstructorKind;
use crate::object::property::Key;
use crate::realm::intrinsics::Intrinsic;
use crate::script::{Compiled, ParsedTree, SourceCopy, vec_bytes};
use crate::value::{ObjectRef, RealmRef, StringRef, Value};

/// What the steps of PerformEval before the evaluation answer.
pub(crate) enum EvalStart {
    /// The value of the `eval`: its argument, when it is no String and the host gives no code
    /// for it (step 3), or `undefined` for a source with no statement (step 11).
    Done(Value),
    /// The eval code, in the script record `script`, and whether it is strict (strictEval).
    Code {
        script: Handle,
        compiled: Rc<Compiled>,
        strict: bool,
    },
}

/// The kind of CreateDynamicFunction (`#sec-createdynamicfunction`, steps 2 to 5): the prefix
/// of the source text, the fallback prototype and the parse entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DynamicFunctionKind {
    /// `normal`, the `Function` constructor.
    Normal,
    /// `generator`, `%GeneratorFunction%`.
    Generator,
    /// `async`, `%AsyncFunction%`.
    Async,
    /// `async-generator`, `%AsyncGeneratorFunction%`.
    AsyncGenerator,
}

impl DynamicFunctionKind {
    /// The source text up to the parameter text: the prefix, then " anonymous(" (step 15).
    fn head(self) -> &'static str {
        match self {
            DynamicFunctionKind::Normal => "function anonymous(",
            DynamicFunctionKind::Generator => "function* anonymous(",
            DynamicFunctionKind::Async => "async function anonymous(",
            DynamicFunctionKind::AsyncGenerator => "async function* anonymous(",
        }
    }

    /// fallbackProto: `%Function.prototype%`, `%GeneratorFunction.prototype%`,
    /// `%AsyncFunction.prototype%` or `%AsyncGeneratorFunction.prototype%`.
    fn fallback_prototype(self) -> Intrinsic {
        match self {
            DynamicFunctionKind::Normal => Intrinsic::FunctionPrototype,
            DynamicFunctionKind::Generator => Intrinsic::GeneratorFunctionPrototype,
            DynamicFunctionKind::Async => Intrinsic::AsyncFunctionPrototype,
            DynamicFunctionKind::AsyncGenerator => Intrinsic::AsyncGeneratorFunctionPrototype,
        }
    }
}

/// What an eval code's frame takes from the code that called `eval` (PerformEval,
/// `#sec-performeval`; `docs/design/engine.md`, decision 3, "`this` and `arguments` in an arrow
/// function", item 5).
#[derive(Clone, Copy)]
struct EvalCaller {
    /// The caller's ThisBinding, or the global `this` for an indirect `eval`.
    this: Value,
    /// GetThisEnvironment's [[FunctionObject]] for a direct `eval`: the caller frame's function;
    /// `None` for an indirect `eval` and outside function code.
    function: Option<ObjectRef>,
    /// The caller frame's NewTarget for a direct `eval`; `None` for an indirect one.
    new_target: Option<ObjectRef>,
}

/// The environments of an eval code's frame (PerformEval, steps 16 to 18).
#[derive(Clone, Copy)]
struct EvalEnvironments {
    /// The LexicalEnvironment.
    lexical: Option<Handle>,
    /// The VariableEnvironment; `None` is the global environment.
    variable: Option<Handle>,
    /// The LexicalEnvironment of the code that called `eval` (`None` for the global
    /// environment): where EvalDeclarationInstantiation, step 3, starts its walk.
    caller: Option<Handle>,
    /// Whether the eval code is strict.
    strict: bool,
}

impl<H: Host> Agent<H> {
    /// HostEnsureCanCompileStrings (`#sec-hostensurecancompilestrings`) of `request`, the hook's
    /// answer checked (decision 6): a `Throw` holding a stale or foreign handle faults the agent;
    /// a termination of the run while the hook ran is the answer. With `activation`, the hook runs
    /// under it (the module's documentation).
    fn ensure_can_compile_strings(
        &mut self,
        request: &CompileStrings<'_>,
        activation: Option<Activation>,
    ) -> Completion<()> {
        let pushed = activation.is_some();
        if let Some(activation) = activation {
            self.activations.push(activation);
        }
        let start = self.scope.len();
        let answer = H::host_ensure_can_compile_strings(&mut Cx { agent: self }, request);
        self.scope.truncate(start);
        if pushed {
            self.activations.pop();
        }
        // A terminated run ignores what the hook returned (decision 10).
        self.running()?;
        match answer {
            Ok(()) => Ok(()),
            Err(Abrupt::Throw(value)) => {
                if self.heap.check_value(value).is_err() {
                    return Err(self.fault());
                }
                Err(Abrupt::Throw(value))
            }
            Err(Abrupt::Terminated(termination)) => Err(self.terminate(termination)),
        }
    }

    /// HostGetCodeForEval (`#sec-hostgetcodeforeval`) of `object`: the String the host gives,
    /// checked (decision 6: a stale or foreign handle faults the agent), or `None` for no-code.
    fn code_for_eval(&mut self, object: ObjectRef) -> Completion<Option<StringRef>> {
        let start = self.scope.len();
        let answer = H::host_get_code_for_eval(&mut HookCx { agent: self }, object);
        self.scope.truncate(start);
        self.running()?;
        match answer {
            Some(string) => {
                if self.heap.check_value(Value::String(string)).is_err() {
                    return Err(self.fault());
                }
                Ok(Some(string))
            }
            None => Ok(None),
        }
    }

    /// The length in code units of the String `string`.
    fn source_length(&mut self, string: StringRef) -> Completion<usize> {
        match self.heap.strings.get(string.handle) {
            Ok(crate::heap::string::HeapString::Latin1(bytes)) => Ok(bytes.len()),
            Ok(crate::heap::string::HeapString::Utf16(units)) => Ok(units.len()),
            Err(_) => Err(self.fault()),
        }
    }

    /// The code units of `string`, copied for a parse: the copy is charged to the memory budget
    /// until it is kept with the code made from it or credited (`script.rs`, [`SourceCopy`]).
    /// The interrupt flag is read first (decision 12): a set flag is cleared and ends the run in
    /// `Terminated(Interrupted)` before anything is copied.
    fn charged_source(&mut self, string: StringRef) -> Completion<SourceCopy> {
        self.check_interrupt()?;
        let length = self.source_length(string)?;
        let mut copy = self.source_copy(length, |_| {})?;
        match self.heap.strings.get(string.handle) {
            Ok(crate::heap::string::HeapString::Latin1(bytes)) => {
                copy.units.extend(bytes.iter().map(|&b| u16::from(b)));
            }
            Ok(crate::heap::string::HeapString::Utf16(own)) => copy.units.extend_from_slice(own),
            Err(_) => {
                self.credit_source_copy(copy);
                return Err(self.fault());
            }
        }
        Ok(copy)
    }

    /// The throw of the first of the `SyntaxError`s of a parse, in `realm`.
    fn throw_parse_errors(
        &mut self,
        realm: Handle,
        errors: Vec<swell_syntax::SyntaxError>,
    ) -> Abrupt {
        match self.syntax_errors(realm, errors) {
            Ok(objects) => match objects.first() {
                Some(&error) => Abrupt::Throw(error),
                None => self.fault(),
            },
            Err(abrupt) => abrupt,
        }
    }

    /// PerformEval (`#sec-performeval`) of `source` up to its evaluation, steps 2 to 11 and the
    /// parse of steps 9 to 15: HostGetCodeForEval of an object, the return of a value that is no
    /// String, HostEnsureCanCompileStrings, then the parse and compile step of eval code in the
    /// current realm (`compilation`: `DirectEval` with `strict_caller` and the activation the
    /// hook runs under, or `IndirectEval`; `function` the function GetThisEnvironment answers for
    /// a direct `eval`, `None` otherwise). The script record of the code holds `host_defined`.
    fn eval_start(
        &mut self,
        source: Value,
        (strict_caller, function): (bool, Option<ObjectRef>),
        compilation: (CompilationType, Option<Activation>),
        host_defined: Option<crate::host::HostDefined>,
    ) -> Completion<EvalStart> {
        // Steps 2 and 3.
        let string = match source {
            Value::Object(object) => match self.code_for_eval(object)? {
                Some(string) => string,
                None => return Ok(EvalStart::Done(source)),
            },
            Value::String(string) => string,
            _ => return Ok(EvalStart::Done(source)),
        };
        let keep = self.keep(Value::String(string));
        let started = self.eval_code_of(
            string,
            source,
            (strict_caller, function),
            compilation,
            host_defined,
        );
        self.release(keep);
        started
    }

    /// Steps 4 to 15 of PerformEval for the String `string`, made from `source`, the eval context
    /// derived from the caller's strictness and its function ([`Agent::eval_context`]).
    fn eval_code_of(
        &mut self,
        string: StringRef,
        source: Value,
        (strict_caller, function): (bool, Option<ObjectRef>),
        compilation: (CompilationType, Option<Activation>),
        host_defined: Option<crate::host::HostDefined>,
    ) -> Completion<EvalStart> {
        // Step 4: evalRealm, the current realm.
        let realm = self.current_realm()?;
        // Step 6.
        let (kind, activation) = compilation;
        self.ensure_can_compile_strings(
            &CompileStrings {
                callee_realm: RealmRef { handle: realm },
                param_strings: &[],
                body_string: string,
                code_string: string,
                compilation_type: kind,
                param_args: &[],
                body_arg: source,
            },
            activation,
        )?;
        // Steps 7 to 11: the flags; then the parse of steps 12 to 15.
        let context = self.eval_context(strict_caller, function)?;
        let direct = matches!(kind, CompilationType::DirectEval);
        let (script, copy) = self.parse_eval_code(realm, string, context, direct)?;
        // Step 11: a source with no statement.
        if script.body.is_empty() {
            self.drop_tree(script);
            self.credit_source_copy(copy);
            return Ok(EvalStart::Done(Value::Undefined));
        }
        let strict = script.strict;
        let compiled = swell_compiler::compile_eval_with(
            &script,
            context.in_derived_constructor,
            &mut self.ledger(),
        );
        self.drop_tree(script);
        let (code, copy) = match self.compiled_with_source(realm, compiled, copy)? {
            Ok(made) => made,
            Err(error) => return Err(Abrupt::Throw(error)),
        };
        let script = self.make_script_record(realm, code, copy, host_defined, true)?;
        let (_, compiled) = self.script_parts(script)?;
        Ok(EvalStart::Code {
            script,
            compiled,
            strict,
        })
    }

    /// PerformEval, steps 7 to 11 (`#sec-performeval`): the eval context of eval code whose
    /// caller's strictness is `strict_caller` and whose GetThisEnvironment's [[FunctionObject]]
    /// is `function` (decision 3, item 5): inFunc when there is one, inMethod when it has a
    /// [[HomeObject]] (HasSuperBinding), inDerivedCtor when it is a class constructor whose box
    /// says derived ([[ConstructorKind]], `object/class_constructor.rs`).
    /// inClassFieldInitializer stays false until `story:eval-in-class-field-initializers` reads
    /// it from the template kind `FieldInitializer`; until then a direct eval whose function is
    /// a field initializer (an arrow in one too, whose GetThisEnvironment is the initializer's)
    /// is refused with a `SyntaxError`, `eval in a class field initializer is not supported
    /// before slice 8`, so that ContainsArguments is not skipped.
    fn eval_context(
        &mut self,
        strict_caller: bool,
        function: Option<ObjectRef>,
    ) -> Completion<EvalContext> {
        let (in_method, in_derived_constructor, in_initializer) = match function {
            Some(function) => match &self.engine_object(function)?.kind {
                ObjectKind::EcmascriptFunction(f) => (
                    f.home_object.is_some(),
                    f.class
                        .as_ref()
                        .is_some_and(|c| c.kind == ConstructorKind::Derived),
                    f.compiled.as_ref().is_some_and(|c| {
                        usize::try_from(f.template)
                            .ok()
                            .and_then(|t| c.code.functions.get(t))
                            .is_some_and(|t| {
                                t.kind == swell_syntax::ast::FunctionKind::FieldInitializer
                            })
                    }),
                ),
                _ => (false, false, false),
            },
            None => (false, false, false),
        };
        if in_initializer {
            return Err(self.throw_error(
                crate::host::ErrorKind::SyntaxError,
                "eval in a class field initializer is not supported before slice 8",
            ));
        }
        Ok(EvalContext {
            strict_caller,
            in_function: function.is_some(),
            in_method,
            in_derived_constructor,
            in_class_field_initializer: false,
        })
    }

    /// `ParseText(x, Script)` of PerformEval (step 12) in the eval context `context` through
    /// `swell_syntax::parse_eval_with` on the agent's ledger, with the agent's `parser_nesting`:
    /// the script with the charged copy of its source, or the throw of its first `SyntaxError` in
    /// `realm`, the copy credited. A step per code unit of the String is charged before the copy
    /// (`Agent::charge_steps`, which reads the interrupt flag first). Not inlined into the run
    /// loop, whose frame stays as small as it was. For a `direct` eval an error of an undeclared
    /// private name is read by [`Agent::private_names_of_direct_eval`].
    #[inline(never)]
    fn parse_eval_code(
        &mut self,
        realm: Handle,
        string: StringRef,
        context: EvalContext,
        direct: bool,
    ) -> Completion<(swell_syntax::Script, SourceCopy)> {
        let options = ParseOptions {
            max_nesting: self.limits.parser_nesting,
        };
        let length = self.source_length(string)?;
        self.charge_steps(u64::try_from(length).unwrap_or(u64::MAX))?;
        let copy = self.charged_source(string)?;
        let parsed =
            swell_syntax::parse_eval_with(&copy.units, context, options, &mut self.ledger());
        match self.parsed_tree(parsed, copy)? {
            ParsedTree::Tree(script, copy) => Ok((script, copy)),
            ParsedTree::Errors(mut errors) => {
                if direct {
                    self.private_names_of_direct_eval(&mut errors)?;
                }
                Err(self.throw_parse_errors(realm, errors))
            }
        }
    }

    /// The frame of eval code (PerformEval, steps 16 to 26), not yet pushed: the current realm,
    /// the environments `envs`, from `caller` the `this` (the caller's ThisBinding), the function
    /// GetThisEnvironment answers and the NewTarget (decision 3, item 5; the evalContext's own
    /// Function component is null, and nothing reads it but through GetThisEnvironment), and
    /// the code of the script record `script`. Its locals and its own bytes are charged with it.
    fn eval_frame(
        &mut self,
        script: Handle,
        compiled: Rc<Compiled>,
        strict: bool,
        envs: EvalEnvironments,
        caller: EvalCaller,
    ) -> Completion<Frame> {
        let realm = self.current_realm()?;
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
        // EvalDeclarationInstantiation, step 16: the lexical bindings that live in local slots
        // start uninitialized.
        for lexical in &compiled.code.lexical {
            if let Some(Slot::Local(n)) = lexical.slot
                && let Some(slot) = usize::try_from(n).ok().and_then(|n| locals.get_mut(n))
            {
                *slot = SlotValue::Uninitialized;
            }
        }
        Ok(Frame {
            compiled,
            template: None,
            pc: 0,
            stack: Vec::new(),
            locals,
            env: envs.lexical,
            var_env: envs.variable,
            pushed_envs: 0,
            this: caller.this,
            function: caller.function,
            new_target: caller.new_target,
            realm,
            script,
            strict,
            completion: Value::Undefined,
            constructed: Constructed::None,
            charged,
        })
    }

    /// The environments of PerformEval, steps 16 to 18, for the eval code `compiled`, from the
    /// caller's LexicalEnvironment `lexical` and VariableEnvironment `variable`. The new
    /// declarative record of step 16 is made only when a binding lives in it (decision 3,
    /// "Lexical bindings", item 5): with a `let` or `const` binding that lives in a record
    /// ([`swell_compiler::bytecode::Code::eval_scope`]), a record of those slots, uninitialized,
    /// whose [[OuterEnv]] is `lexical`; for strict code always, its VariableEnvironment too, where
    /// its `var` and function bindings are added. Non-strict eval code whose lexical bindings
    /// live in local slots, or which has none, runs in the caller's environments: no code can
    /// tell the record is missing.
    fn eval_environments(
        &mut self,
        compiled: &Rc<Compiled>,
        script: Handle,
        strict: bool,
        (lexical, variable): (Option<Handle>, Option<Handle>),
    ) -> Completion<EvalEnvironments> {
        let own = match compiled.code.eval_scope {
            Some(index) => {
                let slots = compiled
                    .code
                    .scopes
                    .get(usize::try_from(index).unwrap_or(usize::MAX))
                    .map(|names| u32::try_from(names.len()).unwrap_or(u32::MAX));
                let Some(slots) = slots else {
                    return Err(self.fault());
                };
                let names = SlotNames {
                    script,
                    compiled: Rc::clone(compiled),
                    table: NameTable::Scope {
                        template: None,
                        index,
                    },
                };
                Some(self.new_lexical_environment(lexical, slots, names)?)
            }
            None if strict => Some(self.new_declarative_environment(lexical, 0, None)?),
            None => None,
        };
        Ok(EvalEnvironments {
            lexical: own.or(lexical),
            variable: if strict { own } else { variable },
            caller: lexical,
            strict,
        })
    }

    /// The direct `eval` of `CallEval` (`#sec-function-calls-runtime-semantics-evaluation`, step
    /// 6.a: the callee is `%eval%` of the current realm, `source` is the first argument):
    /// PerformEval(`source`, `strict_caller`, direct-eval). `Done` when no code is to run;
    /// otherwise the frame of the eval code, whose environments and `this` are the running
    /// frame's, EvalDeclarationInstantiation done, pushed on the running loop.
    pub(crate) fn direct_eval(
        &mut self,
        eval: ObjectRef,
        source: Value,
        strict_caller: bool,
    ) -> Completion<Option<Value>> {
        let (script, realm, lexical, variable, caller) = {
            let frame = self.top_frame()?;
            (
                frame.script,
                frame.realm,
                frame.env,
                frame.var_env,
                EvalCaller {
                    this: frame.this,
                    function: frame.function,
                    new_target: frame.new_target,
                },
            )
        };
        // The evalContext's ScriptOrModule is the caller's (step 22): its [[HostDefined]] is
        // kept by the eval code's own record.
        let host_defined = self.script_record_host_defined(script);
        let activation = Activation {
            callee: eval,
            this: Value::Undefined,
            args: vec![source],
            new_target: None,
            realm: Some(realm),
        };
        let start = self.eval_start(
            source,
            (strict_caller, caller.function),
            (CompilationType::DirectEval, Some(activation)),
            host_defined,
        )?;
        let (script, compiled, strict) = match start {
            EvalStart::Done(value) => return Ok(Some(value)),
            EvalStart::Code {
                script,
                compiled,
                strict,
            } => (script, compiled, strict),
        };
        self.check_frame_depth()?;
        self.step()?;
        let envs = self.eval_environments(&compiled, script, strict, (lexical, variable))?;
        let frame = self.eval_frame(script, Rc::clone(&compiled), strict, envs, caller)?;
        let base = self.interp.frames.len();
        self.push_frame(frame);
        // Step 27, with the eval code's frame running: a throw drops it, before any of its code
        // ran, so no handler of the eval code catches it.
        if let Err(abrupt) = self.eval_declaration_instantiation(&compiled, script, envs) {
            self.drop_frames(base);
            self.set_realm_slot(Some(realm));
            return Err(abrupt);
        }
        self.check_interrupt()?;
        Ok(None)
    }

    /// `eval ( x )` (`#sec-eval-x`): PerformEval(x, false, indirect-eval). The eval code runs in
    /// a loop of its own, a re-entry, in the global environment of the current realm with its
    /// global `this`; its script record has no [[HostDefined]] (the running execution context
    /// is the builtin's, whose ScriptOrModule is null).
    pub(crate) fn indirect_eval(&mut self, args: &[Value]) -> Completion<Value> {
        let source = argument(args, 0);
        let start = self.eval_start(
            source,
            (false, None),
            (CompilationType::IndirectEval, None),
            None,
        )?;
        let (script, compiled, strict) = match start {
            EvalStart::Done(value) => return Ok(value),
            EvalStart::Code {
                script,
                compiled,
                strict,
            } => (script, compiled, strict),
        };
        let entered = self.enter_code(None)?;
        let result = self.run_indirect_eval(script, compiled, strict);
        self.leave_code(entered);
        result
    }

    /// The frame of an indirect `eval`'s code, run until it returns.
    fn run_indirect_eval(
        &mut self,
        script: Handle,
        compiled: Rc<Compiled>,
        strict: bool,
    ) -> Completion<Value> {
        self.check_frame_depth()?;
        self.step()?;
        let realm = self.current_realm()?;
        let this = self.global_this_binding(realm)?;
        let envs = self.eval_environments(&compiled, script, strict, (None, None))?;
        let caller = EvalCaller {
            this,
            function: None,
            new_target: None,
        };
        let frame = self.eval_frame(script, Rc::clone(&compiled), strict, envs, caller)?;
        self.run_frame(frame, |agent| {
            agent.eval_declaration_instantiation(&compiled, script, envs)
        })
    }

    /// EvalDeclarationInstantiation (`#sec-evaldeclarationinstantiation`) of the eval code
    /// `compiled` of script record `script`, with the code's `var` names and function
    /// declarations as the compiler took them from `parse_eval` (`swell-compiler`,
    /// `compile_eval`), in the environments `envs`. A step per check and per declaration.
    ///
    /// - Step 3, for non-strict code only, before anything is bound: on a global
    ///   VariableEnvironment, a `SyntaxError` when the global environment's [[DeclarativeRecord]]
    ///   binds a `var` or function name (3.a); then the walk from the caller's LexicalEnvironment
    ///   out to the VariableEnvironment (3.b to 3.d), passing over the records of `with`
    ///   statements, a `SyntaxError` when a record binds the name as a `let` or `const` binding
    ///   (or one eval code added). The binding of a `catch` parameter passes, as the normative
    ///   optional rule for VariableStatements in Catch Blocks (B.3.4), which a web host
    ///   supports, has it. A step per name and record.
    /// - The block-level function declarations of B.3.2.3 (step 12) are slice 18.
    /// - Steps 8 and 10 on a global VariableEnvironment: CanDeclareGlobalFunction and
    ///   CanDeclareGlobalVar, a `TypeError` when one answers false.
    /// - Step 16: the lexical bindings were created uninitialized with the frame
    ///   (`eval_environments`, `eval_frame`); their declarations initialize them.
    /// - Steps 17 and 18: each function made in the LexicalEnvironment and bound, each `var`
    ///   bound to `undefined` unless bound, the bindings deletable: CreateGlobalFunctionBinding
    ///   and CreateGlobalVarBinding with `true` on a global VariableEnvironment; on a declarative
    ///   one, a binding the record has (a named slot or one an earlier eval added) is set, any
    ///   other is added at run time (`env/mod.rs`, [`Agent::add_binding`]).
    fn eval_declaration_instantiation(
        &mut self,
        compiled: &Rc<Compiled>,
        script: Handle,
        envs: EvalEnvironments,
    ) -> Completion<()> {
        let code = &compiled.code;
        let realm = self.current_realm()?;
        if !envs.strict {
            self.eval_var_conflicts(compiled, realm, envs)?;
        }
        if envs.variable.is_none() {
            // Step 8: the functions, in reverse List order.
            for &function in code.function_declarations.iter().rev() {
                self.step()?;
                let name = self.name_key_of(compiled, function)?;
                if !self.can_declare_global_function(realm, name)? {
                    return Err(self.throw_error(
                        crate::host::ErrorKind::TypeError,
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
                        crate::host::ErrorKind::TypeError,
                        "a global variable cannot be declared on this global object",
                    ));
                }
            }
        }
        // Step 17.
        for &function in &code.function_declarations {
            self.step()?;
            let name = self.name_key_of(compiled, function)?;
            let made =
                self.instantiate_function(compiled, script, realm, function, envs.lexical)?;
            let keep = self.keep(Value::Object(made));
            let bound = match envs.variable {
                None => self.create_global_function_binding(realm, name, Value::Object(made), true),
                Some(env) => {
                    if self.declarative_has_binding(env, name)? {
                        self.set_declarative_binding(env, name, Value::Object(made))
                    } else {
                        self.add_binding(env, name, Value::Object(made))
                    }
                }
            };
            self.release(keep);
            bound?;
        }
        // Step 18.
        for &variable in &code.var_names {
            self.step()?;
            let name = self.string_key_of(compiled, variable)?;
            match envs.variable {
                None => self.create_global_var_binding(realm, name, true)?,
                Some(env) => {
                    if !self.declarative_has_binding(env, name)? {
                        self.add_binding(env, name, Value::Undefined)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// EvalDeclarationInstantiation, step 3 (the documentation of
    /// [`Agent::eval_declaration_instantiation`]): no `var` or function name of non-strict eval
    /// code may be bound by a lexical declaration it would be hoisted over.
    fn eval_var_conflicts(
        &mut self,
        compiled: &Rc<Compiled>,
        realm: Handle,
        envs: EvalEnvironments,
    ) -> Completion<()> {
        let code = &compiled.code;
        let mut names = Vec::new();
        for &function in &code.function_declarations {
            names.push(self.name_key_of(compiled, function)?);
        }
        for &variable in &code.var_names {
            names.push(self.string_key_of(compiled, variable)?);
        }
        for name in names {
            // Step 3.a.
            if envs.variable.is_none() {
                self.step()?;
                if self.has_lexical_declaration(realm, name)? {
                    return Err(self.already_declared(name));
                }
            }
            // Steps 3.b to 3.d.
            let mut this = envs.caller;
            while this != envs.variable {
                let Some(env) = this else {
                    break;
                };
                self.step()?;
                let (outer, with, parameters) = {
                    let record = self.environment(env)?;
                    let parameters = matches!(
                        &record.names,
                        Some(RecordNames::Slots(SlotNames {
                            table: NameTable::Function(_),
                            ..
                        }))
                    );
                    (record.outer, record.binding_object.is_some(), parameters)
                };
                if !with {
                    match self.bound(env, name)? {
                        Some(Bound::Slot(_, SlotKind::Let | SlotKind::Const) | Bound::Added(_)) => {
                            return Err(self.already_declared(name));
                        }
                        // The environment of the parameters of a function whose parameters
                        // have an initializer, between the eval code of one of them and the
                        // record its `var` names go to (`#sec-functiondeclarationinstantiation`,
                        // step 20): any binding of it conflicts.
                        Some(Bound::Slot(_, SlotKind::Mutable)) if parameters => {
                            return Err(self.already_declared(name));
                        }
                        Some(Bound::Slot(_, SlotKind::Mutable | SlotKind::SelfName)) | None => {}
                    }
                }
                this = outer;
            }
        }
        Ok(())
    }

    /// `Function ( ...parameterArgs, bodyArg )` (`#sec-function-p1-p2-pn-body`): the active
    /// function `constructor`, `bodyArg` the last argument or the empty String, then
    /// CreateDynamicFunction(constructor, newTarget, normal, parameterArgs, bodyArg).
    pub(crate) fn function_constructor(
        &mut self,
        constructor: ObjectRef,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        let (params, body) = match args.split_last() {
            Some((body, params)) => (params, Some(*body)),
            None => (args, None),
        };
        let new_target = new_target.unwrap_or(constructor);
        let made =
            self.create_dynamic_function(new_target, DynamicFunctionKind::Normal, params, body)?;
        Ok(Value::Object(made))
    }

    /// The source String of a parameter or of the body of CreateDynamicFunction (steps 9 and
    /// 10): HostGetCodeForEval of an object, else (or for no-code) ToString.
    fn dynamic_function_part(&mut self, arg: Value) -> Completion<StringRef> {
        if let Value::Object(object) = arg
            && let Some(string) = self.code_for_eval(object)?
        {
            return Ok(string);
        }
        self.to_string(arg)
    }

    /// CreateDynamicFunction (`#sec-createdynamicfunction`) of `kind`, with `new_target` set
    /// (step 1), the parameter arguments `params` and the body argument `body` (`None`: the
    /// empty String). The parts' Strings, the source String and the function are kept across
    /// every step that may run code.
    pub(crate) fn create_dynamic_function(
        &mut self,
        new_target: ObjectRef,
        kind: DynamicFunctionKind,
        params: &[Value],
        body: Option<Value>,
    ) -> Completion<ObjectRef> {
        // The list of paramStrings, charged before it is made and credited once it is dropped.
        let list = vec_bytes::<StringRef>(params.len());
        if self.heap.accounting.charge(list).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        let made = self.with_kept_room(|agent, room| {
            agent.dynamic_function_steps(new_target, kind, params, body, room)
        });
        self.heap.accounting.credit(list);
        made
    }

    /// The steps of [`Agent::create_dynamic_function`]; what it keeps is released by the caller,
    /// the paramStrings in `room`.
    fn dynamic_function_steps(
        &mut self,
        new_target: ObjectRef,
        kind: DynamicFunctionKind,
        params: &[Value],
        body: Option<Value>,
        room: &mut KeptRoom,
    ) -> Completion<ObjectRef> {
        // Steps 9 and 10: paramStrings, then bodyString, each kept.
        let mut param_strings = Vec::new();
        #[expect(
            clippy::disallowed_methods,
            reason = "charged by `create_dynamic_function` before these steps, `vec_bytes::<StringRef>` of the parameters"
        )]
        if param_strings.try_reserve_exact(params.len()).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        for &arg in params {
            let string = self.dynamic_function_part(arg)?;
            self.reserve_kept(room, 1)?;
            self.keep(Value::String(string));
            param_strings.push(string);
        }
        let body_arg = match body {
            Some(value) => value,
            None => Value::String(self.new_string_str("")?),
        };
        self.keep(body_arg);
        let body_string = self.dynamic_function_part(body_arg)?;
        self.keep(Value::String(body_string));
        // Step 11.
        let realm = self.current_realm()?;
        // Steps 12 to 15: P, bodyParseString and sourceString, whose length `string_length`
        // bounds and whose code units are charged a step each as they are copied.
        let mut builder = StringBuilder::new();
        self.append_str(&mut builder, kind.head())?;
        for (k, &string) in param_strings.iter().enumerate() {
            if k > 0 {
                self.append_str(&mut builder, ",")?;
            }
            self.append_string(&mut builder, string)?;
        }
        self.append_str(&mut builder, "\n) {\n")?;
        self.append_string(&mut builder, body_string)?;
        self.append_str(&mut builder, "\n}")?;
        let source_string = self.finish_string(builder)?;
        self.keep(Value::String(source_string));
        // Step 16: the compilation type is `Function` for every kind.
        self.ensure_can_compile_strings(
            &CompileStrings {
                callee_realm: RealmRef { handle: realm },
                param_strings: &param_strings,
                body_string,
                code_string: source_string,
                compilation_type: CompilationType::Function,
                param_args: params,
                body_arg,
            },
            None,
        )?;
        // Steps 17 to 25: the three parses, then the compile step. The text of the last parse is
        // kept with the code: the function's [[SourceText]] (step 28) is all of it.
        let (script, copy) =
            self.parse_dynamic_function_code(realm, kind, source_string, &param_strings)?;
        let compiled = swell_compiler::compile_script_with(&script, &mut self.ledger());
        self.drop_tree(script);
        let (mut code, copy) = match self.compiled_with_source(realm, compiled, copy)? {
            Ok(made) => made,
            Err(error) => return Err(Abrupt::Throw(error)),
        };
        // Step 30: SetFunctionName(F, "anonymous"), the name of the function's template; the new
        // name is charged with the code before it is made, and the old one credited.
        let units = "anonymous".len();
        let bytes = units.saturating_mul(size_of::<u16>());
        let charged = self.heap.accounting.charge(vec_bytes::<u16>(units)).is_ok();
        let mut name = Vec::new();
        #[expect(
            clippy::disallowed_methods,
            reason = "charged by the statement above, `vec_bytes::<u16>` of the name, before it is reserved"
        )]
        if code.functions.is_empty() || !charged || name.try_reserve_exact(units).is_err() {
            let fault = code.functions.is_empty();
            if charged {
                self.heap.accounting.credit(vec_bytes::<u16>(units));
            }
            self.drop_code(code);
            self.credit_source_copy(copy);
            return Err(if fault {
                self.fault()
            } else {
                self.terminate(Termination::MemoryBudget)
            });
        }
        name.extend("anonymous".encode_utf16());
        if let Some(template) = code.functions.first_mut() {
            let old = core::mem::replace(&mut template.name, name);
            let old_bytes = old.capacity().saturating_mul(size_of::<u16>());
            drop(old);
            code.charged = code.charged.saturating_sub(old_bytes).saturating_add(bytes);
            self.heap
                .accounting
                .credit(u64::try_from(old_bytes).unwrap_or(u64::MAX));
        }
        let script = self.make_script_record(realm, code, copy, None, true)?;
        let (_, compiled) = self.script_parts(script)?;
        // Steps 27 to 32: OrdinaryFunctionCreate in the global environment of the realm, with
        // its name, and MakeConstructor for kind `normal`; for kind `generator` the template is
        // a generator's, so the function has no [[Construct]] and its `prototype` is a new
        // object whose [[Prototype]] is `%GeneratorPrototype%`; for kind `async` the template
        // is an `async` function's, so the function has neither [[Construct]] nor `prototype`;
        // for kind `async-generator` the template is an async generator's, so the function has
        // no [[Construct]] and its `prototype` is a new object whose [[Prototype]] is
        // `%AsyncGeneratorPrototype%` (`instantiate_function`); then the [[Prototype]] step 26
        // gives it.
        let function = self.instantiate_function(&compiled, script, realm, 0, None)?;
        self.keep(Value::Object(function));
        // Step 26: GetPrototypeFromConstructor(newTarget, fallbackProto), which may run code; it
        // is read once the source parsed, as the steps order it.
        let prototype =
            self.get_prototype_from_constructor(new_target, kind.fallback_prototype())?;
        self.engine_object_mut(function)?.prototype = Some(prototype);
        Ok(function)
    }

    /// The parses of CreateDynamicFunction (steps 17 to 25) through the entry of `kind` with
    /// the agent's `parser_nesting` (`swell_syntax::parse_dynamic_function_with`,
    /// `parse_dynamic_generator_function_with`, `parse_dynamic_async_function_with` or
    /// `parse_dynamic_async_generator_function_with`): the parameter text and the body text, taken
    /// from the source String `source` (the head of `kind`, P, `\n) {\n`, the body, `\n}`), whose
    /// copy is charged from the start; the third parse reads the same text, so the spans of its
    /// tree are spans of the copy. The script with the copy, or the throw of the first
    /// `SyntaxError` in `realm` (a refusal the parse held back to its end among them, counted
    /// as a script's is), the copy credited. Not inlined into its caller.
    #[inline(never)]
    fn parse_dynamic_function_code(
        &mut self,
        realm: Handle,
        kind: DynamicFunctionKind,
        source: StringRef,
        param_strings: &[StringRef],
    ) -> Completion<(swell_syntax::Script, SourceCopy)> {
        let head = kind.head().len();
        let mut joined = param_strings.len().saturating_sub(1);
        for &string in param_strings {
            let length = self.source_length(string)?;
            joined = joined.saturating_add(length);
        }
        let options = ParseOptions {
            max_nesting: self.limits.parser_nesting,
        };
        let copy = self.charged_source(source)?;
        let units = &copy.units;
        let body_start = head.saturating_add(joined).saturating_add("\n) {\n".len());
        let parts = (
            units.get(head..head.saturating_add(joined)),
            units.get(body_start..units.len().saturating_sub("\n}".len())),
        );
        let entry = match kind {
            DynamicFunctionKind::Normal => swell_syntax::parse_dynamic_function_with,
            DynamicFunctionKind::Generator => swell_syntax::parse_dynamic_generator_function_with,
            DynamicFunctionKind::Async => swell_syntax::parse_dynamic_async_function_with,
            DynamicFunctionKind::AsyncGenerator => {
                swell_syntax::parse_dynamic_async_generator_function_with
            }
        };
        let parsed = match parts {
            (Some(parameters), Some(body)) => {
                Some(entry(parameters, body, options, &mut self.ledger()))
            }
            _ => None,
        };
        let Some(parsed) = parsed else {
            self.credit_source_copy(copy);
            return Err(self.fault());
        };
        match self.parsed_tree(parsed, copy)? {
            ParsedTree::Tree(script, copy) => Ok((script, copy)),
            ParsedTree::Errors(errors) => Err(self.throw_parse_errors(realm, errors)),
        }
    }
}

/// The text of the early error AllPrivateIdentifiersValid gives for `#x` in eval code
/// (`swell_syntax`, `early.rs`): `private name #x is not declared in an enclosing class`.
const UNDECLARED_PRIVATE_NAME: (&str, &str) =
    ("private name ", " is not declared in an enclosing class");

impl<H: Host> Agent<H> {
    /// Direct eval code is parsed with the private names of the classes around the call
    /// (`#sec-performeval`, step 12: privateIdentifiers, the names of the running
    /// PrivateEnvironment and its outer ones), which `story:private-names-in-eval-code` brings.
    /// Until then the parse knows none of them and gives AllPrivateIdentifiersValid's error for
    /// each private name of the code. Read here, after a parse that failed, for each such error:
    /// - the name is declared by a class whose body encloses the call when a declarative record
    ///   on the chain of the running frame's LexicalEnvironment names a slot `#x`
    ///   ([`Agent::chain_names_slot`]): the class's private record, whose slots are named
    ///   whenever a direct `eval` stands in the class body (`swell-compiler`, `resolve.rs`,
    ///   `LexicalScope::named`), entered after the heritage;
    /// - when every error of the parse is of such a name, the standard runs the code and the
    ///   engine cannot: the first becomes the one error, the refusal `private name #x in direct
    ///   eval code is not supported before slice 8` (a `Refusal` by `swell_syntax::message_kind`);
    /// - otherwise the errors of declared names are no errors of the standard and are dropped;
    ///   the others stay, the first of them thrown. A name no enclosing class declares, inside a
    ///   class or outside every class, keeps its `SyntaxError`.
    ///
    /// A direct `eval` whose code parses never comes here, so one that names no `#` pays
    /// nothing: no walk, no allocation. The name is looked up as an atom that already exists
    /// (`Heap::atom_of`, which makes nothing): when there is none, no table names it.
    fn private_names_of_direct_eval(
        &mut self,
        errors: &mut Vec<swell_syntax::SyntaxError>,
    ) -> Completion<()> {
        // The direct `eval` pushed no frame before its parse: the running frame is the caller's.
        let env = self.top_frame()?.env;
        let (head, tail) = UNDECLARED_PRIVATE_NAME;
        let mut declared = 0usize;
        for error in errors.iter_mut() {
            let Some(name) = error
                .message
                .strip_prefix(head)
                .and_then(|rest| rest.strip_suffix(tail))
            else {
                continue;
            };
            if !self.enclosing_class_declares(env, name)? {
                continue;
            }
            let refusal = alloc::format!(
                "private name {name} in direct eval code is not supported before slice 8"
            );
            error.message = refusal;
            declared += 1;
        }
        if declared == 0 {
            return Ok(());
        }
        if declared == errors.len() {
            errors.truncate(1);
        } else {
            errors.retain(|e| {
                !matches!(
                    swell_syntax::message_kind(&e.message),
                    swell_syntax::MessageKind::Refusal { .. }
                )
            });
        }
        Ok(())
    }

    /// Whether a class whose body encloses the running code declares the private name `name`
    /// (its text, with its `#`): a slot of that name in a record on the chain from `env`.
    fn enclosing_class_declares(&mut self, env: Option<Handle>, name: &str) -> Completion<bool> {
        let atom = if name.is_ascii() {
            self.heap.atom_of(CodeUnits::Latin1(name.as_bytes()))
        } else {
            let mut units = ChargedVec::new();
            self.reserve_vec(&mut units, name.encode_utf16().count())?;
            for unit in name.encode_utf16() {
                if units.push(unit).is_err() {
                    return Err(self.fault());
                }
            }
            self.heap.atom_of(&units[..])
        };
        match atom {
            Some(atom) => self.chain_names_slot(env, Key::String(atom)),
            None => Ok(false),
        }
    }
}
