//! ECMAScript function objects (ECMA-262 10.2, `#sec-ecmascript-function-objects`):
//! [[Environment]], [[Realm]], [[ScriptOrModule]], [[HomeObject]] and the compiled code;
//! OrdinaryFunctionCreate (`#sec-ordinaryfunctioncreate`) with SetFunctionLength, SetFunctionName
//! and MakeConstructor (`#sec-makeconstructor`), [[Call]] (10.2.1) and [[Construct]] (10.2.2) from
//! Rust, PrepareForOrdinaryCall, OrdinaryCallBindThis (`#sec-ordinarycallbindthis`) and
//! FunctionDeclarationInstantiation (`#sec-functiondeclarationinstantiation`) with the
//! arguments object (`object/arguments.rs`). `story:object-model-and-realms` wrote the dispatch
//! arms of `object/mod.rs`; `story:interpreter-and-script-entry-points` the rest;
//! `story:with-arguments-and-dynamic-names` the arguments object and the names of the
//! environments a call and a named function expression make; `story:arrow-functions` the `this`
//! an arrow function keeps and the environment of an arrow function that calls `eval`.
//!
//! A function's code is a template of its script's compiled code (`swell-compiler`'s
//! `FunctionTemplate`), whose constants the script record holds: the function reports its
//! script to the collector, which keeps them. A call from ECMAScript code pushes a frame in the
//! run loop (`interp/dispatch.rs`); [`call`] and [`construct`] are the internal methods called
//! from Rust, each a re-entry (`interp/mod.rs`).
//!
//! FunctionDeclarationInstantiation is split as the compiler split it: the call stores the
//! arguments in the parameters' slots (step 22, "IteratorBindingInitialization" of a simple
//! parameter list, a later parameter of a name overwriting an earlier one), then the
//! arguments object in the slot of `arguments` when the template asks for one (steps 15 to 22:
//! no code runs between, so the order of the two is not observable), every other binding
//! starts as `undefined` (steps 27 and 28), and the function's code begins with the
//! instantiation of its function declarations (step 36).

use std::rc::Rc;

use swell_syntax::ast::FunctionKind;

use crate::env::{NameTable, SlotNames};
use crate::heap::arena::Handle;
use crate::heap::collect::{Item, Mark};
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::{Completion, Termination};
use crate::interp::frame::{Frame, SlotValue, frame_bytes};
use crate::object::ObjectKind;
use crate::object::arguments::Mapping;
use crate::object::builtin_function::Activation;
use crate::object::ordinary::OrdinaryData;
use crate::object::property::Property;
use crate::realm::intrinsics::Intrinsic;
use crate::script::Compiled;
use crate::value::{ObjectRef, Value};
use swell_compiler::bytecode::Slot;

/// The slots of an ECMAScript function object (10.2, Table 28). Every field is `Some` for a
/// function made by [`Agent::instantiate_function`]; the record of the tests of the collector
/// has none.
pub(crate) struct EcmascriptFunction {
    /// [[Environment]]: a record of the `environments` arena, or `None` for the global
    /// environment of [[Realm]].
    pub(crate) environment: Option<Handle>,
    /// [[Realm]].
    pub(crate) realm: Option<Handle>,
    /// [[ScriptOrModule]].
    pub(crate) script: Option<Handle>,
    /// [[HomeObject]]: the object of an accessor or a plain method of an object literal
    /// (MakeMethod, `#sec-makemethod`).
    pub(crate) home_object: Option<ObjectRef>,
    /// The compiled code of [[ScriptOrModule]], whose template `template` is [[ECMAScriptCode]]
    /// and [[FormalParameters]].
    pub(crate) compiled: Option<Rc<Compiled>>,
    pub(crate) template: u32,
    /// Whether it has a [[Construct]] (MakeConstructor): a function declaration or expression,
    /// not an accessor or a method.
    pub(crate) constructor: bool,
    /// [[Strict]]; [[ThisMode]] is strict exactly when it is, but for an arrow function.
    pub(crate) strict: bool,
    /// Of an arrow function, whose [[ThisMode]] is lexical: the `this` of the code that made it,
    /// which a call puts in its frame in place of OrdinaryCallBindThis (`docs/design/engine.md`,
    /// decision 3, "`this` and `arguments` in an arrow function", item 1). `None` for every
    /// other function.
    pub(crate) lexical_this: Option<Value>,
}

impl EcmascriptFunction {
    /// Kind site of the collector: [[Environment]], [[Realm]], [[ScriptOrModule]] (which holds
    /// the code's constants), [[HomeObject]] and the `this` an arrow function keeps.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/ecmascript_function.rs EcmascriptFunction::trace");
        if let Some(environment) = self.environment {
            mark.push(Item::Environment(environment));
        }
        if let Some(realm) = self.realm {
            mark.push(Item::Realm(realm));
        }
        if let Some(script) = self.script {
            mark.push(Item::Script(script));
        }
        if let Some(home) = self.home_object {
            mark.object(home);
        }
        if let Some(this) = self.lexical_this {
            mark.value(this);
        }
    }

    /// An ECMAScript function object has a [[Call]].
    pub(crate) fn has_call(&self) -> bool {
        true
    }

    /// Whether it has a [[Construct]].
    pub(crate) fn has_construct(&self) -> bool {
        self.constructor
    }

    /// An ECMAScript function, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> EcmascriptFunction {
        EcmascriptFunction {
            environment: None,
            realm: None,
            script: None,
            home_object: None,
            compiled: None,
            template: 0,
            constructor: false,
            strict: false,
            lexical_this: None,
        }
    }
}

/// What a call needs of a function object.
struct Callee {
    environment: Option<Handle>,
    realm: Handle,
    script: Handle,
    compiled: Rc<Compiled>,
    template: u32,
    strict: bool,
    lexical_this: Option<Value>,
}

/// [[Call]] (`#sec-ecmascript-function-objects-call-thisargument-argumentslist`) from Rust: a
/// re-entry, run in a loop of its own.
pub(crate) fn call<H: Host>(
    agent: &mut Agent<H>,
    function: ObjectRef,
    this: Value,
    args: &[Value],
) -> Completion<Value> {
    let realm = agent.function_parts(function)?.realm;
    let entered = agent.enter_code(Some((
        Activation {
            callee: function,
            this,
            args: Vec::new(),
            new_target: None,
            realm: Some(realm),
        },
        args,
    )))?;
    let result = agent.call_function(function, this, args, None);
    agent.leave_code(entered);
    result
}

/// [[Construct]] (`#sec-ecmascript-function-objects-construct-argumentslist-newtarget`) from
/// Rust, of a base constructor (every constructor of slice 1): `this` is
/// OrdinaryCreateFromConstructor(newTarget, "%Object.prototype%"), and the call answers it unless
/// the code returns an object.
pub(crate) fn construct<H: Host>(
    agent: &mut Agent<H>,
    function: ObjectRef,
    args: &[Value],
    new_target: ObjectRef,
) -> Completion<Value> {
    let realm = agent.function_parts(function)?.realm;
    let entered = agent.enter_code(Some((
        Activation {
            callee: function,
            this: Value::Undefined,
            args: Vec::new(),
            new_target: Some(new_target),
            realm: Some(realm),
        },
        args,
    )))?;
    let result = agent.construct_function(function, args, new_target);
    agent.leave_code(entered);
    result
}

impl<H: Host> Agent<H> {
    /// The slots a call needs of the ECMAScript function `function`.
    fn function_parts(&mut self, function: ObjectRef) -> Completion<Callee> {
        let parts = match &self.engine_object(function)?.kind {
            ObjectKind::EcmascriptFunction(f) => match (f.realm, f.script, &f.compiled) {
                (Some(realm), Some(script), Some(compiled)) => Some(Callee {
                    environment: f.environment,
                    realm,
                    script,
                    compiled: Rc::clone(compiled),
                    template: f.template,
                    strict: f.strict,
                    lexical_this: f.lexical_this,
                }),
                _ => None,
            },
            _ => None,
        };
        match parts {
            Some(parts) => Ok(parts),
            None => Err(self.fault()),
        }
    }

    /// [[Call]] of `function` in a loop of its own: the frame, then the run.
    fn call_function(
        &mut self,
        function: ObjectRef,
        this: Value,
        args: &[Value],
        constructed: Option<ObjectRef>,
    ) -> Completion<Value> {
        self.check_frame_depth()?;
        self.step()?;
        let frame = self.make_frame(function, this, args, constructed)?;
        self.run_frame(frame, |_| Ok(()))
    }

    /// [[Construct]] of `function` in a loop of its own.
    fn construct_function(
        &mut self,
        function: ObjectRef,
        args: &[Value],
        new_target: ObjectRef,
    ) -> Completion<Value> {
        self.check_frame_depth()?;
        let this = self.ordinary_create_from_constructor(
            new_target,
            Intrinsic::ObjectPrototype,
            OrdinaryData::Plain,
        )?;
        let keep = self.keep(Value::Object(this));
        let result = self.call_function(function, Value::Undefined, args, Some(this));
        self.release(keep);
        result
    }

    /// The frame of a call of `function` (PrepareForOrdinaryCall, `#sec-prepareforordinarycall`,
    /// OrdinaryCallBindThis and the parameters of FunctionDeclarationInstantiation), not yet
    /// pushed. `constructed` is the `this` of a [[Construct]]. Runs no code; its allocations
    /// are charged, the frame's own bytes with it.
    pub(crate) fn make_frame(
        &mut self,
        function: ObjectRef,
        this_argument: Value,
        args: &[Value],
        constructed: Option<ObjectRef>,
    ) -> Completion<Frame> {
        let callee = self.function_parts(function)?;
        let index = usize::try_from(callee.template).unwrap_or(usize::MAX);
        let Some(template) = callee.compiled.code.functions.get(index) else {
            return Err(self.fault());
        };
        let locals = template.locals;
        let captured = u32::try_from(template.captured.len()).unwrap_or(u32::MAX);
        let binds = template.binds_parameters;
        let parameter_environment = template.parameter_environment;
        let environment = captured > 0 || template.eval_environment || parameter_environment;
        let params = &template.params;
        let arguments = template.arguments;

        // An arrow function has no [[Construct]] and binds no `this` (OrdinaryCallBindThis,
        // step 2): its frame takes the value the function kept.
        let this = match (constructed, callee.lexical_this) {
            (Some(object), _) => Value::Object(object),
            (None, Some(this)) => this,
            (None, None) => self.ordinary_call_bind_this(&callee, this_argument)?,
        };
        let (env, var_env) = if environment {
            // Steps 19 and 20: the parameters' environment of non-strict code whose
            // initializers call `eval` directly lies inside a record of its own, the
            // VariableEnvironment until the code pushes the body's (`PushVarEnv`).
            let outer = if parameter_environment {
                Some(self.new_declarative_environment(callee.environment, 0, None)?)
            } else {
                callee.environment
            };
            let names = SlotNames {
                script: callee.script,
                compiled: Rc::clone(&callee.compiled),
                table: NameTable::Function(callee.template),
            };
            let env = Some(self.new_function_environment(outer, captured, names)?);
            (env, if parameter_environment { outer } else { env })
        } else {
            (callee.environment, callee.environment)
        };

        // A list that is not simple keeps the arguments in the slots after the frame's own,
        // charged with the frame and traced with its locals, for the code that binds the
        // parameters (`binds_parameters`).
        let kept = if binds { args.len() } else { 0 };
        let count = usize::try_from(locals)
            .unwrap_or(usize::MAX)
            .saturating_add(kept);
        let local_bytes =
            u64::try_from(count.saturating_mul(size_of::<Value>())).unwrap_or(u64::MAX);
        let charged = frame_bytes().saturating_add(local_bytes);
        if self.heap.accounting.charge(charged).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        let mut slots = Vec::new();
        if slots.try_reserve_exact(count).is_err() {
            self.heap.accounting.credit(charged);
            return Err(self.terminate(Termination::MemoryBudget));
        }
        slots.resize(
            count.saturating_sub(kept),
            SlotValue::Value(Value::Undefined),
        );
        if binds {
            slots.extend(args.iter().map(|&value| SlotValue::Value(value)));
        }

        // FunctionDeclarationInstantiation, step 22: each argument in its parameter's slot; for
        // a list that is not simple each parameter uninitialized until its code binds it.
        let stored = if binds {
            self.uninitialize_parameters(params, env, &mut slots)
        } else {
            self.bind_parameters(params, args, env, &mut slots)
        };
        if !stored {
            self.heap.accounting.credit(charged);
            return Err(self.fault());
        }

        // Steps 15 to 22: the arguments object, once the parameters hold their values.
        if let Some(arguments) = arguments {
            let mapping = arguments.mapped.then_some(Mapping { env, params });
            let made = self.instantiate_arguments(
                function,
                args,
                (callee.realm, mapping),
                (arguments.slot, env),
                &mut slots,
            );
            if let Err(abrupt) = made {
                self.heap.accounting.credit(charged);
                return Err(abrupt);
            }
        }

        Ok(Frame {
            compiled: callee.compiled,
            template: Some(callee.template),
            pc: 0,
            stack: Vec::new(),
            locals: slots,
            env,
            var_env,
            pushed_envs: 0,
            this,
            function: Some(function),
            realm: callee.realm,
            script: callee.script,
            strict: callee.strict,
            completion: Value::Undefined,
            constructed,
            charged,
        })
    }

    /// FunctionDeclarationInstantiation, step 22, of a simple list: each of `args` in its
    /// parameter's slot of `params`, a local of `locals` or a slot of the function environment
    /// `env`, a missing one `undefined`; whether every slot was there.
    #[inline(never)]
    fn bind_parameters(
        &mut self,
        params: &[Slot],
        args: &[Value],
        env: Option<Handle>,
        locals: &mut [SlotValue],
    ) -> bool {
        for (position, slot) in params.iter().enumerate() {
            let value = args.get(position).copied().unwrap_or(Value::Undefined);
            let stored = match *slot {
                Slot::Local(n) => usize::try_from(n)
                    .ok()
                    .and_then(|n| locals.get_mut(n))
                    .map(|binding| *binding = SlotValue::Value(value))
                    .is_some(),
                Slot::Env(n) => self.set_slot(env, 0, n, value).is_ok(),
            };
            if !stored {
                return false;
            }
        }
        true
    }

    /// Steps 21 and 26 of a list that is not simple: each parameter's slot of `params`, a
    /// local of `locals` or a slot of the function environment `env`, uninitialized until the
    /// code binds it in order; whether every slot was there.
    #[inline(never)]
    fn uninitialize_parameters(
        &mut self,
        params: &[Slot],
        env: Option<Handle>,
        locals: &mut [SlotValue],
    ) -> bool {
        for slot in params {
            let cleared = match *slot {
                Slot::Local(n) => usize::try_from(n).ok().and_then(|n| locals.get_mut(n)),
                Slot::Env(n) => env.and_then(|handle| {
                    let record = self.heap.environments.get_mut(handle).ok()?;
                    record.slots.get_mut(usize::try_from(n).ok()?)
                }),
            }
            .map(|binding| *binding = SlotValue::Uninitialized);
            if cleared.is_none() {
                return false;
            }
        }
        true
    }

    /// FunctionDeclarationInstantiation, steps 15 to 22: the arguments object of a call of
    /// `function` with `args` stored in `slot`, a local of `locals` or a slot of the function
    /// environment of `mapping`. Not inlined into the call, whose frame stays as small as it
    /// was.
    #[inline(never)]
    fn instantiate_arguments(
        &mut self,
        function: ObjectRef,
        args: &[Value],
        mapping: (Handle, Option<Mapping<'_>>),
        slot: (Slot, Option<Handle>),
        locals: &mut [SlotValue],
    ) -> Completion<()> {
        let (realm, mapping) = mapping;
        let object = self.create_arguments_object(realm, function, args, mapping)?;
        let value = Value::Object(object);
        match slot {
            (Slot::Local(n), _) => match usize::try_from(n).ok().and_then(|n| locals.get_mut(n)) {
                Some(binding) => {
                    *binding = SlotValue::Value(value);
                    Ok(())
                }
                None => Err(self.fault()),
            },
            (Slot::Env(n), env) => self.set_slot(env, 0, n, value),
        }
    }

    /// OrdinaryCallBindThis (`#sec-ordinarycallbindthis`): strict code takes `this` as given;
    /// otherwise `undefined` and `null` become the global `this` of the callee's realm and
    /// another value ToObject of it, whose wrapper is made in the callee's realm.
    fn ordinary_call_bind_this(
        &mut self,
        callee: &Callee,
        this_argument: Value,
    ) -> Completion<Value> {
        if callee.strict {
            return Ok(this_argument);
        }
        match this_argument {
            Value::Undefined | Value::Null => self.global_this_binding(callee.realm),
            Value::Object(_) => Ok(this_argument),
            _ => {
                let saved = self.realm_slot();
                self.set_realm_slot(Some(callee.realm));
                let wrapped = self.to_object(this_argument);
                self.set_realm_slot(saved);
                Ok(Value::Object(wrapped?))
            }
        }
    }

    /// InstantiateOrdinaryFunctionObject (`#sec-runtime-semantics-instantiateordinaryfunctionobject`),
    /// InstantiateOrdinaryFunctionExpression
    /// (`#sec-runtime-semantics-instantiateordinaryfunctionexpression`) or the closure of an
    /// accessor's or a plain method's MethodDefinitionEvaluation
    /// (`#sec-runtime-semantics-methoddefinitionevaluation`, `#sec-runtime-semantics-definemethod`)
    /// of template `index` of `compiled`, in the environment `env`: OrdinaryFunctionCreate with
    /// `%Function.prototype%` of `realm` (`#sec-ordinaryfunctioncreate`: SetFunctionLength,
    /// `#sec-setfunctionlength`), SetFunctionName (`#sec-setfunctionname`), and MakeConstructor
    /// (`#sec-makeconstructor`) but for an accessor, a method or an arrow function, which have no
    /// [[Construct]] (an arrow function's `this` is set by [`Agent::keep_lexical_this`] after).
    /// A generator template, whatever its kind, is InstantiateGeneratorFunctionObject
    /// (`#sec-runtime-semantics-instantiategeneratorfunctionobject`),
    /// InstantiateGeneratorFunctionExpression
    /// (`#sec-runtime-semantics-instantiategeneratorfunctionexpression`) or the
    /// MethodDefinitionEvaluation of a GeneratorMethod
    /// (`#sec-runtime-semantics-methoddefinitionevaluation`): `%GeneratorFunction.prototype%`
    /// of `realm` instead of `%Function.prototype%`, no [[Construct]], and a `prototype`
    /// { [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]: false } holding a new
    /// object whose [[Prototype]] is `%GeneratorPrototype%`, with no `constructor`.
    /// An `async` template, an arrow included, is InstantiateAsyncFunctionObject
    /// (`#sec-runtime-semantics-instantiateasyncfunctionobject`),
    /// InstantiateAsyncFunctionExpression
    /// (`#sec-runtime-semantics-instantiateasyncfunctionexpression`) or the evaluation of an
    /// async arrow (`#sec-async-arrow-function-definitions-runtime-semantics-evaluation`):
    /// `%AsyncFunction.prototype%` of `realm`, no [[Construct]] and no `prototype`.
    /// An async generator template (both `generator` and `asynchronous`) is
    /// InstantiateAsyncGeneratorFunctionObject
    /// (`#sec-runtime-semantics-instantiateasyncgeneratorfunctionobject`) or
    /// InstantiateAsyncGeneratorFunctionExpression
    /// (`#sec-runtime-semantics-instantiateasyncgeneratorfunctionexpression`): as a generator's,
    /// with `%AsyncGeneratorFunction.prototype%` and `%AsyncGeneratorPrototype%` of `realm`.
    /// A named function expression that refers to its own name gets an environment of one
    /// immutable binding holding it.
    pub(crate) fn instantiate_function(
        &mut self,
        compiled: &Rc<Compiled>,
        script: Handle,
        realm: Handle,
        index: u32,
        env: Option<Handle>,
    ) -> Completion<ObjectRef> {
        let position = usize::try_from(index).unwrap_or(usize::MAX);
        let (template, name) = match (
            compiled.code.functions.get(position),
            compiled.names.get(position),
        ) {
            (Some(template), Some(&name)) => (template, name),
            _ => return Err(self.fault()),
        };
        let generator = template.generator;
        let asynchronous = template.asynchronous;
        let constructor = !generator
            && !asynchronous
            && matches!(
                template.kind,
                FunctionKind::Declaration | FunctionKind::Expression
            );
        let self_binding = template.self_binding;
        let length = template.length;
        let strict = template.strict;

        let environment = if self_binding {
            let names = SlotNames {
                script,
                compiled: Rc::clone(compiled),
                table: NameTable::SelfName(index),
            };
            Some(self.new_declarative_environment(env, 1, Some(names))?)
        } else {
            env
        };
        let prototype = if generator && asynchronous {
            self.intrinsic(realm, Intrinsic::AsyncGeneratorFunctionPrototype)?
        } else if generator {
            self.intrinsic(realm, Intrinsic::GeneratorFunctionPrototype)?
        } else if asynchronous {
            self.intrinsic(realm, Intrinsic::AsyncFunctionPrototype)?
        } else {
            self.intrinsic(realm, Intrinsic::FunctionPrototype)?
        };
        let function = self.allocate_object(
            Some(prototype),
            ObjectKind::EcmascriptFunction(EcmascriptFunction {
                environment,
                realm: Some(realm),
                script: Some(script),
                home_object: None,
                compiled: Some(Rc::clone(compiled)),
                template: index,
                constructor,
                strict,
                lexical_this: None,
            }),
        )?;
        if self_binding {
            self.set_slot(environment, 0, 0, Value::Object(function))?;
        }
        let length_key = self.name_key("length")?;
        self.add_property(
            function,
            length_key,
            Property::data(Value::Number(f64::from(length)), false, false, true),
        )?;
        let name_key = self.name_key("name")?;
        self.add_property(
            function,
            name_key,
            Property::data(Value::String(name), false, false, true),
        )?;
        if constructor {
            // MakeConstructor: a new prototype object whose "constructor" is the function.
            let object_prototype = self.intrinsic(realm, Intrinsic::ObjectPrototype)?;
            let made = self.ordinary_object_create(Some(object_prototype), OrdinaryData::Plain)?;
            let constructor_key = self.name_key("constructor")?;
            self.add_property(
                made,
                constructor_key,
                Property::data(Value::Object(function), true, false, true),
            )?;
            let prototype_key = self.name_key("prototype")?;
            self.add_property(
                function,
                prototype_key,
                Property::data(Value::Object(made), true, false, false),
            )?;
        }
        if generator {
            // A new object whose [[Prototype]] is %GeneratorPrototype%, or
            // %AsyncGeneratorPrototype% for an async generator, with no "constructor", as the
            // function's "prototype".
            let generator_prototype = if asynchronous {
                self.intrinsic(realm, Intrinsic::AsyncGeneratorPrototype)?
            } else {
                self.intrinsic(realm, Intrinsic::GeneratorPrototype)?
            };
            let made =
                self.ordinary_object_create(Some(generator_prototype), OrdinaryData::Plain)?;
            let prototype_key = self.name_key("prototype")?;
            self.add_property(
                function,
                prototype_key,
                Property::data(Value::Object(made), true, false, false),
            )?;
        }
        Ok(function)
    }

    /// MakeMethod (`#sec-makemethod`): [[HomeObject]] of the accessor or method `function` is
    /// `home`.
    pub(crate) fn make_method(&mut self, function: ObjectRef, home: ObjectRef) -> Completion<()> {
        match &mut self.engine_object_mut(function)?.kind {
            ObjectKind::EcmascriptFunction(f) => {
                f.home_object = Some(home);
                Ok(())
            }
            _ => Err(self.fault()),
        }
    }

    /// The `this` the arrow function `function` keeps (`#sec-runtime-semantics-instantiatearrowfunctionexpression`,
    /// [[ThisMode]] lexical): the value of the frame that made it. Allocates nothing.
    pub(crate) fn keep_lexical_this(&mut self, function: ObjectRef, this: Value) -> Completion<()> {
        match &mut self.engine_object_mut(function)?.kind {
            ObjectKind::EcmascriptFunction(f) => {
                f.lexical_this = Some(this);
                Ok(())
            }
            _ => Err(self.fault()),
        }
    }

    /// Whether `function` is an ECMAScript function object, which a call from the run loop
    /// pushes a frame for.
    pub(crate) fn is_ecmascript_function(&mut self, function: ObjectRef) -> Completion<bool> {
        Ok(matches!(
            self.engine_object(function)?.kind,
            ObjectKind::EcmascriptFunction(_)
        ))
    }
}

#[cfg(test)]
thread_local! {
    /// The argument values copied for the activation of a call from Rust ([`call`] and
    /// [`construct`]), counted where [`Agent::enter_code`] copies them, in the counting build of
    /// the tests; a test resets it before it reads it.
    pub(crate) static CALL_ARGUMENTS_COPIED: std::cell::Cell<u64> =
        const { std::cell::Cell::new(0) };
    /// The values the `Call(n)` and `New(n)` instructions copied off the operand stack, on
    /// both paths, in the counting build of the tests; a test resets it before it reads it.
    pub(crate) static INSTRUCTION_ARGUMENTS_COPIED: std::cell::Cell<u64> =
        const { std::cell::Cell::new(0) };
}

#[cfg(test)]
mod tests {
    use super::CALL_ARGUMENTS_COPIED;
    use crate::host::agent::Agent;
    use crate::host::hooks::test_host::{NoHooks, agent_with_realm_and, enter, limits};
    use crate::object::property::Key;
    use crate::value::{Root, Value};
    use crate::{Outcome, Parsed, Realm, Script, Source, Termination};

    /// The number of arguments of the calls of these tests.
    const N: u32 = 100_000;

    /// `source` parsed in `realm`.
    fn parse(agent: &mut Agent<NoHooks>, realm: &Realm, source: &str) -> Script {
        match agent.parse_script(realm, Source::Utf8(source), None) {
            Ok(Parsed::Parsed(script)) => script,
            other => panic!("{source} did not parse: {other:?}"),
        }
    }

    /// An agent with no step budget to speak of and a memory budget of what its realm holds
    /// plus 256 MiB, which that limit is, whose global `list` is an Array of the Numbers 0 to
    /// `count - 1`, made from Rust, after `setup` ran in it normally.
    fn roomy_after(count: u32, setup: &str) -> (Agent<NoHooks>, Realm, u64) {
        let mut limits = limits();
        limits.steps_per_entry = u64::MAX;
        limits.memory_bytes = 1 << 30;
        let (mut agent, realm) = agent_with_realm_and(limits);
        let roomy_limit = agent.heap.bytes() + (1 << 28);
        agent.heap.accounting.set_limit(roomy_limit);
        enter(&mut agent, &realm);
        let list = agent.array_create(0.0, None).unwrap();
        for i in 0..count {
            assert_eq!(
                agent.create_data_property(list, Key::Index(i), Value::Number(f64::from(i))),
                Ok(true)
            );
        }
        let global = agent.global_object(&realm).unwrap();
        let Ok(Value::Object(global)) = agent.heap.roots.get(&global) else {
            panic!("the global object");
        };
        let key = agent.name_key("list").unwrap();
        assert_eq!(
            agent.create_data_property(global, key, Value::Object(list)),
            Ok(true)
        );
        let script = parse(&mut agent, &realm, setup);
        let outcome = agent.evaluate_script(&script);
        assert!(matches!(outcome, Ok(Outcome::Normal(_))), "{outcome:?}");
        (agent, realm, roomy_limit)
    }

    /// The value `root` holds.
    fn value(agent: &Agent<NoHooks>, root: &Root) -> Value {
        match agent.heap.roots.get(root) {
            Ok(value) => value,
            Err(error) => panic!("the root answered {error:?}"),
        }
    }

    /// `docs/design/engine.md`, "Memory budget" (storage is charged before it is allocated and
    /// credited where its holder drops it), and [[Call]] of an ECMAScript function
    /// (`#sec-ecmascript-function-objects-call-thisargument-argumentslist`, step 2, the push of
    /// calleeContext): `function f() {}` bound with 100,000 arguments through
    /// `Function.prototype.bind.apply` and called as `b()` by a script, whose run loop has no
    /// builtin between it and the call, so the call records its arguments in an activation.
    /// Under a budget of the reading after a collection plus the gathered list's bytes plus
    /// half as much again the script ends in `Terminated(MemoryBudget)` before an argument is
    /// copied. With the budget raised it answers normally and copies each argument once, and
    /// after a collection the reading is the one before.
    #[test]
    fn the_arguments_a_call_records_are_charged_before_they_are_copied() {
        let setup = "function f() {} var b = Function.prototype.bind.apply(f, list);";
        let (mut agent, realm, roomy_limit) = roomy_after(N + 1, setup);
        let script = parse(&mut agent, &realm, "b()");
        assert!(agent.collect().is_ok());
        let before = agent.heap_bytes().unwrap();
        let list = u64::from(N) * size_of::<Value>() as u64;
        agent.heap.accounting.set_limit(before + list + list / 2);
        CALL_ARGUMENTS_COPIED.with(|copied| copied.set(0));
        let outcome = agent.evaluate_script(&script);
        assert!(
            matches!(outcome, Ok(Outcome::Terminated(Termination::MemoryBudget))),
            "{outcome:?}"
        );
        assert_eq!(
            CALL_ARGUMENTS_COPIED.with(std::cell::Cell::get),
            0,
            "arguments copied before the memory budget ended the run"
        );
        agent.heap.accounting.set_limit(roomy_limit);
        let outcome = agent.evaluate_script(&script);
        assert!(matches!(outcome, Ok(Outcome::Normal(_))), "{outcome:?}");
        drop(outcome);
        assert_eq!(
            CALL_ARGUMENTS_COPIED.with(std::cell::Cell::get),
            u64::from(N)
        );
        assert!(agent.collect().is_ok());
        assert_eq!(agent.heap_bytes().unwrap(), before, "after the call");
    }

    /// `docs/design/engine.md`, "Memory budget" (what is not kept is not copied), and
    /// `Function.prototype.apply` (`#sec-function.prototype.apply`): `f.apply(null, list)` with
    /// 100,000 values under a roomy budget answers `undefined`, and the call of `f`, reached
    /// from the builtin's activation, which records the call, copies none of its arguments.
    #[test]
    fn a_call_that_records_nothing_copies_nothing() {
        let (mut agent, realm, _) = roomy_after(N, "function f() {}");
        let script = parse(&mut agent, &realm, "f.apply(null, list)");
        CALL_ARGUMENTS_COPIED.with(|copied| copied.set(0));
        let outcome = agent.evaluate_script(&script);
        let Ok(Outcome::Normal(answer)) = outcome else {
            panic!("{outcome:?}");
        };
        assert_eq!(value(&agent, &answer), Value::Undefined);
        assert_eq!(
            CALL_ARGUMENTS_COPIED.with(std::cell::Cell::get),
            0,
            "arguments copied for a call that records none"
        );
    }

    /// `docs/design/engine.md`, "Memory budget" and "Collector" (a credit does not lower the
    /// bytes counted since the last collection, so a transient charge brings a collection
    /// forward): 12,500 calls by a script of a function bound with eight arguments, each call
    /// recording its eight arguments in an activation, 100,000 values charged in all, under a
    /// roomy budget run at most one more collection per `MIN_TRIGGER_BYTES` of the arguments
    /// the activations charge than the collections counted on the base of the change, which
    /// charged nothing for them: 5 there, 7 with the charge. Not with the feature
    /// `collect-every-safe-point`, where every safe point collects.
    #[cfg(not(feature = "collect-every-safe-point"))]
    #[test]
    fn a_loop_of_bound_calls_collects_within_its_bound() {
        use crate::heap::{COLLECTIONS, MIN_TRIGGER_BYTES};
        const ON_THE_BASE: u64 = 5;
        const SOURCE: &str = "function f() {} var b8 = f.bind(null, 1, 2, 3, 4, 5, 6, 7, 8); \
            for (var i = 0; i < 12500; i++) b8(); i";
        let (mut agent, realm, _) = roomy_after(0, "");
        let script = parse(&mut agent, &realm, SOURCE);
        COLLECTIONS.with(|collections| collections.set(0));
        CALL_ARGUMENTS_COPIED.with(|copied| copied.set(0));
        let outcome = agent.evaluate_script(&script);
        let Ok(Outcome::Normal(answer)) = outcome else {
            panic!("{outcome:?}");
        };
        assert_eq!(value(&agent, &answer), Value::Number(12_500.0));
        assert_eq!(
            CALL_ARGUMENTS_COPIED.with(std::cell::Cell::get),
            u64::from(N),
            "the values the activations recorded"
        );
        let collections = COLLECTIONS.with(std::cell::Cell::get);
        let charged = u64::from(N) * size_of::<Value>() as u64;
        let bound = ON_THE_BASE + charged.div_ceil(MIN_TRIGGER_BYTES);
        assert!(
            collections <= bound,
            "{collections} collections, at most {bound}"
        );
    }
}
