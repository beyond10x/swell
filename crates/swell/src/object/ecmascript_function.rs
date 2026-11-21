//! ECMAScript function objects (ECMA-262 10.2, `#sec-ecmascript-function-objects`):
//! [[Environment]], [[Realm]], [[ScriptOrModule]], [[HomeObject]] and the compiled code;
//! OrdinaryFunctionCreate (`#sec-ordinaryfunctioncreate`) with SetFunctionLength, SetFunctionName
//! and MakeConstructor (`#sec-makeconstructor`), [[Call]] (10.2.1) and [[Construct]] (10.2.2) from
//! Rust, PrepareForOrdinaryCall, OrdinaryCallBindThis (`#sec-ordinarycallbindthis`) and
//! FunctionDeclarationInstantiation (`#sec-functiondeclarationinstantiation`) without an
//! arguments object, which the parser refuses until slice 2. `story:object-model-and-realms`
//! wrote the dispatch arms of `object/mod.rs`; `story:interpreter-and-script-entry-points` the
//! rest.
//!
//! A function's code is a template of its script's compiled code (`swell-compiler`'s
//! `FunctionTemplate`), whose constants the script record holds: the function reports its
//! script to the collector, which keeps them. A call from ECMAScript code pushes a frame in the
//! run loop (`interp/dispatch.rs`); [`call`] and [`construct`] are the internal methods called
//! from Rust, each a re-entry (`interp/mod.rs`).
//!
//! FunctionDeclarationInstantiation is split as the compiler split it: the call stores the
//! arguments in the parameters' slots (step 22, "IteratorBindingInitialization" of a simple
//! parameter list, a later parameter of a name overwriting an earlier one), every other binding
//! starts as `undefined` (steps 27 and 28), and the function's code begins with the
//! instantiation of its function declarations (step 36).

use std::rc::Rc;

use swell_syntax::ast::FunctionKind;

use crate::heap::arena::Handle;
use crate::heap::collect::{Item, Mark};
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::{Completion, Termination};
use crate::interp::frame::{Frame, frame_bytes};
use crate::object::ObjectKind;
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
    /// [[HomeObject]]: the object of an accessor of an object literal (MakeMethod,
    /// `#sec-makemethod`).
    pub(crate) home_object: Option<ObjectRef>,
    /// The compiled code of [[ScriptOrModule]], whose template `template` is [[ECMAScriptCode]]
    /// and [[FormalParameters]].
    pub(crate) compiled: Option<Rc<Compiled>>,
    pub(crate) template: u32,
    /// Whether it has a [[Construct]] (MakeConstructor): a function declaration or expression,
    /// not an accessor.
    pub(crate) constructor: bool,
    /// [[Strict]]; [[ThisMode]] is strict exactly when it is.
    pub(crate) strict: bool,
}

impl EcmascriptFunction {
    /// Kind site of the collector: [[Environment]], [[Realm]], [[ScriptOrModule]] (which holds
    /// the code's constants) and [[HomeObject]].
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
    let entered = agent.enter_code(Some(Activation {
        callee: function,
        this,
        args: args.to_vec(),
        new_target: None,
        realm: Some(realm),
    }))?;
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
    let entered = agent.enter_code(Some(Activation {
        callee: function,
        this: Value::Undefined,
        args: args.to_vec(),
        new_target: Some(new_target),
        realm: Some(realm),
    }))?;
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
        let params = template.params.clone();

        let this = match constructed {
            Some(object) => Value::Object(object),
            None => self.ordinary_call_bind_this(&callee, this_argument)?,
        };
        let env = if captured > 0 {
            Some(self.new_function_environment(callee.environment, captured)?)
        } else {
            callee.environment
        };

        let count = usize::try_from(locals).unwrap_or(usize::MAX);
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
        slots.resize(count, Value::Undefined);

        // FunctionDeclarationInstantiation, step 22: each argument in its parameter's slot.
        for (position, slot) in params.iter().enumerate() {
            let value = args.get(position).copied().unwrap_or(Value::Undefined);
            let stored = match *slot {
                Slot::Local(n) => usize::try_from(n)
                    .ok()
                    .and_then(|n| slots.get_mut(n))
                    .map(|binding| *binding = value)
                    .is_some(),
                Slot::Env(n) => self.set_slot(env, 0, n, value).is_ok(),
            };
            if !stored {
                self.heap.accounting.credit(charged);
                return Err(self.fault());
            }
        }

        Ok(Frame {
            compiled: callee.compiled,
            template: Some(callee.template),
            pc: 0,
            stack: Vec::new(),
            locals: slots,
            env,
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
    /// accessor's MethodDefinitionEvaluation (`#sec-runtime-semantics-methoddefinitionevaluation`)
    /// of template `index` of `compiled`, in the environment `env`: OrdinaryFunctionCreate with
    /// `%Function.prototype%` of `realm` (`#sec-ordinaryfunctioncreate`: SetFunctionLength,
    /// `#sec-setfunctionlength`), SetFunctionName (`#sec-setfunctionname`), and MakeConstructor
    /// (`#sec-makeconstructor`) but for an accessor. A named function expression that refers to
    /// its own name gets an environment of one immutable binding holding it.
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
        let constructor = matches!(
            template.kind,
            FunctionKind::Declaration | FunctionKind::Expression
        );
        let self_binding = template.self_binding;
        let length = template.length;
        let strict = template.strict;

        let environment = if self_binding {
            Some(self.new_declarative_environment(env, 1)?)
        } else {
            env
        };
        let prototype = self.intrinsic(realm, Intrinsic::FunctionPrototype)?;
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
        Ok(function)
    }

    /// MakeMethod (`#sec-makemethod`): [[HomeObject]] of the accessor `function` is `home`.
    pub(crate) fn make_method(&mut self, function: ObjectRef, home: ObjectRef) -> Completion<()> {
        match &mut self.engine_object_mut(function)?.kind {
            ObjectKind::EcmascriptFunction(f) => {
                f.home_object = Some(home);
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
