//! Built-in function objects (ECMA-262 10.3, `#sec-built-in-function-objects`): [[Realm]],
//! [[InitialName]], [[Call]] (10.3.1) and [[Construct]] (10.3.2) of builtins and of host
//! functions (`docs/design/host-interface.md`, decision 14), CreateBuiltinFunction (10.3.4), and
//! the activations of running builtins. Written by `story:object-model-and-realms`.
//!
//! Every call into a function goes through [`Agent::builtin_call`] or the arms of
//! `object/mod.rs`: it charges a step, counts against `reentry_depth`, pushes an activation that
//! holds the callee, `this`, the arguments and `new_target`, and is a safe point (decision 4) once
//! the activation holds them. A builtin that keeps a value across a call into code puts it on the
//! agent's kept stack ([`Agent::keep`]), never only in a Rust local. A holder that reserves room
//! on the kept stack for many values (a sort's list, a list of keys) charges the growth before
//! it is made and, when it ends, gives the stack back the room it had and credits the growth
//! ([`KeptRoom`]).
//!
//! A host function's `Ok` and `Abrupt::Throw` values are checked before the engine uses them
//! (decision 6): a stale handle, a handle of another agent, or a constructor answer that is not
//! an object ends the run in `Terminated(EngineFault)`. What a host function returns in a run that
//! is terminated is not used, and not checked.
//!
//! Two sites of the collector: the kind site [`BuiltinFunction::trace`] and the root site
//! [`trace_roots`] (the activations and the kept stack).

use std::any::Any;

use super::ObjectKind;
use super::property::Property;
use crate::heap::arena::Handle;
use crate::heap::collect::{Item, Mark};
use crate::host::agent::Agent;
use crate::host::host_object::{Brand, HostData, HostObject};
use crate::host::outcome::{Abrupt, Completion};
use crate::host::{Cx, ErrorKind, Host, HostCall, HostFn, Termination};
use crate::realm::intrinsics::Intrinsic;
use crate::value::{ObjectRef, StringRef, Value};

/// The builtins of this part, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BuiltinId {
    /// `%Function.prototype%` (20.2.3): "accepts any arguments and returns undefined".
    FunctionPrototype,
    /// `%ThrowTypeError%` (10.2.4.1).
    ThrowTypeError,
    /// `Error` and the NativeError constructors (20.5.1.1, 20.5.6.1.1).
    ErrorConstructor(ErrorKind),
    /// `Error.prototype.toString` (20.5.3.4).
    ErrorPrototypeToString,
    /// `Error.isError` (20.5.2.1), `story:global-this-uri-functions-and-is-error`.
    ErrorIsError,
    /// The builtins of `story:harness-builtins`, each module of `builtins/` with its own ids.
    Object(crate::builtins::object::ObjectBuiltin),
    Function(crate::builtins::function::FunctionBuiltin),
    Array(crate::builtins::array::ArrayBuiltin),
    String(crate::builtins::string::StringBuiltin),
    Math(crate::builtins::math::MathBuiltin),
    Json(crate::builtins::json::JsonBuiltin),
    /// The builtins slice 2 takes early (`story:number-boolean-and-global-functions-early`),
    /// each module of `builtins/` with its own ids.
    Number(crate::builtins::number::NumberBuiltin),
    Boolean(crate::builtins::boolean::BooleanBuiltin),
    Global(crate::builtins::global::GlobalBuiltin),
}

/// What a built-in function does when called.
pub(crate) enum Behaviour {
    Builtin(BuiltinId),
    /// A host function: its `HostFn<H>` and its host data.
    Host {
        steps: Box<dyn Any>,
        data: Option<HostObject>,
    },
}

/// The slots of a built-in function object.
pub(crate) struct BuiltinFunction {
    /// [[Realm]].
    pub(crate) realm: Option<Handle>,
    /// [[InitialName]].
    pub(crate) initial_name: Option<StringRef>,
    pub(crate) behaviour: Behaviour,
    /// Whether it has a [[Construct]].
    pub(crate) constructor: bool,
}

impl BuiltinFunction {
    /// Kind site of the collector: [[Realm]], [[InitialName]], and what a host function's data
    /// lists.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/builtin_function.rs BuiltinFunction::trace");
        if let Some(realm) = self.realm {
            mark.push(Item::Realm(realm));
        }
        if let Some(name) = self.initial_name {
            mark.value(Value::String(name));
        }
        if let Behaviour::Host {
            data: Some(data), ..
        } = &self.behaviour
        {
            data.trace(mark);
        }
    }

    /// A built-in function object, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> BuiltinFunction {
        BuiltinFunction {
            realm: None,
            initial_name: None,
            behaviour: Behaviour::Builtin(BuiltinId::FunctionPrototype),
            constructor: false,
        }
    }
}

/// Host data of a host function, kept as the data of a [`HostObject`] so that its `trace` is
/// called the one way host data is.
struct FunctionData(Box<dyn HostData>);

impl HostData for FunctionData {
    fn trace(&self, tracer: &mut crate::host::host_object::Tracer<'_>) {
        self.0.trace(tracer);
    }
}

/// The room a holder made on the kept stack (a sort's list, a list of keys and what is kept
/// with it): the capacity the stack had before ([`Agent::kept_room`]) and the bytes its growth
/// was charged to the memory budget ([`Agent::reserve_kept`]), which
/// [`Agent::end_kept_room`] gives back and credits.
pub(crate) struct KeptRoom {
    pub(crate) capacity: usize,
    pub(crate) charged: u64,
}

/// The execution context of a running builtin (9.4): what the collector keeps while it runs.
pub(crate) struct Activation {
    pub(crate) callee: ObjectRef,
    pub(crate) this: Value,
    pub(crate) args: Vec<Value>,
    pub(crate) new_target: Option<ObjectRef>,
    /// The callee's [[Realm]], the current realm while it runs.
    pub(crate) realm: Option<Handle>,
}

/// Root site of the collector: the execution contexts, that is what the running builtins hold,
/// their activations and the values they keep across calls into code, and the realm of the
/// entry.
pub(crate) fn trace_roots<H: Host>(agent: &Agent<H>, mark: &mut Mark) {
    mark.count("object/builtin_function.rs trace_roots");
    for activation in &agent.activations {
        mark.object(activation.callee);
        mark.value(activation.this);
        for &arg in &activation.args {
            mark.value(arg);
        }
        if let Some(new_target) = activation.new_target {
            mark.object(new_target);
        }
        if let Some(realm) = activation.realm {
            mark.push(Item::Realm(realm));
        }
    }
    for &value in &agent.kept {
        mark.value(value);
    }
    // The realm the entry names (decision 7) is held by the host's `Realm` for the whole entry,
    // but the one `create_realm` sets is not yet: it is reported here, under the activations.
    if let Some(realm) = agent.entry_realm {
        mark.push(Item::Realm(realm));
    }
}

impl<H: Host> Agent<H> {
    /// Keep `value` alive until [`Agent::release`] of the mark this answers.
    pub(crate) fn keep(&mut self, value: Value) -> usize {
        let mark = self.kept.len();
        self.kept.push(value);
        mark
    }

    /// Drop what was kept since `mark`.
    pub(crate) fn release(&mut self, mark: usize) {
        self.kept.truncate(mark);
    }

    /// The room of the kept stack now, before a holder reserves more on it.
    pub(crate) fn kept_room(&self) -> KeptRoom {
        KeptRoom {
            capacity: self.kept.capacity(),
            charged: 0,
        }
    }

    /// Room on the kept stack for `more` values beyond its length: when it has none, its
    /// capacity grows to twice what it was, at least 16 and at least what is needed, charged
    /// to the memory budget before it is made and added to `room`; over the budget the run
    /// ends in `Terminated(MemoryBudget)`.
    pub(crate) fn reserve_kept(&mut self, room: &mut KeptRoom, more: usize) -> Completion<()> {
        let needed = self.kept.len().saturating_add(more);
        let capacity = self.kept.capacity();
        if needed <= capacity {
            return Ok(());
        }
        let wanted = capacity.saturating_mul(2).max(16).max(needed);
        let slots = u64::try_from(wanted - capacity).unwrap_or(u64::MAX);
        let slot = u64::try_from(size_of::<Value>()).unwrap_or(u64::MAX);
        let bytes = slots.saturating_mul(slot);
        if self.heap.accounting.charge(bytes).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        let length = self.kept.len();
        if self.kept.try_reserve_exact(wanted - length).is_err() {
            self.heap.accounting.credit(bytes);
            return Err(self.terminate(Termination::MemoryBudget));
        }
        room.charged = room.charged.saturating_add(bytes);
        Ok(())
    }

    /// The end of a holder of the kept stack, whatever its completion: what was kept since
    /// `mark` is released, the kept stack is given back the room it had when `room` was taken,
    /// and what its growth was charged is credited.
    pub(crate) fn end_kept_room(&mut self, mark: usize, room: &KeptRoom) {
        self.release(mark);
        let capacity = room.capacity.max(self.kept.len());
        self.kept.shrink_to(capacity);
        self.heap.accounting.credit(room.charged);
    }

    /// CreateBuiltinFunction (`#sec-createbuiltinfunction`) with SetFunctionLength
    /// (`#sec-setfunctionlength`) and SetFunctionName (`#sec-setfunctionname`): `length` then
    /// `name`, each { [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: true }.
    /// `prototype` is `%Function.prototype%` of `realm` when `None`.
    pub(crate) fn create_builtin_function(
        &mut self,
        behaviour: Behaviour,
        length: u32,
        name: &str,
        realm: Handle,
        prototype: Option<ObjectRef>,
        constructor: bool,
    ) -> Completion<ObjectRef> {
        let prototype = match prototype {
            Some(prototype) => prototype,
            None => self.intrinsic(realm, Intrinsic::FunctionPrototype)?,
        };
        let name = self.new_string_str(name)?;
        let keep = self.keep(Value::String(name));
        let made = self.allocate_object(
            Some(prototype),
            ObjectKind::BuiltinFunction(BuiltinFunction {
                realm: Some(realm),
                initial_name: Some(name),
                behaviour,
                constructor,
            }),
        );
        self.release(keep);
        let function = made?;
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
        Ok(function)
    }

    /// A host function (decision 14): CreateBuiltinFunction with the host's steps and data. The
    /// boxes of its steps and of its data (the box of `FunctionData` and what the host's box
    /// holds) are charged to the memory budget before the engine boxes anything; over the budget
    /// the run ends in `Terminated(MemoryBudget)`. The function records them in [`Grown`], so
    /// the sweep that frees it credits them; when no function is made they are credited at once.
    ///
    /// [`Grown`]: super::Grown
    pub(crate) fn create_host_function(
        &mut self,
        realm: Handle,
        name: &str,
        length: u32,
        steps: HostFn<H>,
        constructor: bool,
        data: Option<Box<dyn HostData>>,
    ) -> Completion<ObjectRef> {
        let data_bytes = data.as_deref().map_or(0, |data| {
            size_of::<FunctionData>().saturating_add(size_of_val(data))
        });
        let bytes =
            u64::try_from(size_of::<HostFn<H>>().saturating_add(data_bytes)).unwrap_or(u64::MAX);
        if self.heap.accounting.charge(bytes).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        let data = data.map(|data| HostObject {
            brand: Brand(0),
            data: Some(Box::new(FunctionData(data))),
            error_kind: None,
        });
        let made = self.create_builtin_function(
            Behaviour::Host {
                steps: Box::new(steps),
                data,
            },
            length,
            name,
            realm,
            None,
            constructor,
        );
        let recorded = made.and_then(|function| {
            self.record_grown(function, bytes)?;
            Ok(function)
        });
        if recorded.is_err() {
            self.heap.accounting.credit(bytes);
        }
        recorded
    }

    /// The bookkeeping of every call into a function (decision 2): the sticky termination, a
    /// step, and the re-entry count, whose bound throws a `RangeError` in the current realm.
    fn enter_function(&mut self) -> Completion<()> {
        self.running()?;
        self.charge_steps(1)?;
        let depth = u32::try_from(self.activations.len()).unwrap_or(u32::MAX);
        if depth > self.limits.reentry_depth {
            return Err(self.throw_error(ErrorKind::RangeError, "reentry_depth exceeded"));
        }
        Ok(())
    }

    /// [[Call]] (10.3.1) when `new_target` is `None`, [[Construct]] (10.3.2) otherwise, of the
    /// built-in function object `function`.
    pub(crate) fn builtin_call(
        &mut self,
        function: ObjectRef,
        this: Value,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        let (realm, id) = match &self.engine_object(function)?.kind {
            ObjectKind::BuiltinFunction(builtin) => (
                builtin.realm,
                match builtin.behaviour {
                    Behaviour::Builtin(id) => Some(id),
                    Behaviour::Host { .. } => None,
                },
            ),
            _ => return Err(self.fault()),
        };
        self.enter_function()?;
        self.activations.push(Activation {
            callee: function,
            this,
            args: args.to_vec(),
            new_target,
            realm,
        });
        let sites = self.throw_sites;
        let result = if self.safe_point().is_err() {
            Err(self.fault())
        } else {
            match id {
                Some(id) => self.run_builtin(id, function, this, args, new_target),
                None => self.run_host_function(function, this, args, new_target),
            }
        };
        self.activations.pop();
        // A normal answer after a throw recorded inside the builtin or host function: it caught
        // that throw (decision 13).
        if result.is_ok() && self.throw_sites != sites {
            self.end_throw_site();
        }
        result
    }

    /// The steps of a builtin of this part.
    fn run_builtin(
        &mut self,
        id: BuiltinId,
        function: ObjectRef,
        this: Value,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        match id {
            BuiltinId::FunctionPrototype => Ok(Value::Undefined),
            BuiltinId::ThrowTypeError => Err(self.throw_error(
                ErrorKind::TypeError,
                "'caller', 'callee' and 'arguments' may not be accessed",
            )),
            BuiltinId::ErrorConstructor(kind) => {
                self.error_constructor(kind, function, args, new_target)
            }
            BuiltinId::ErrorPrototypeToString => self.error_prototype_to_string(this),
            BuiltinId::ErrorIsError => self.error_is_error(args),
            BuiltinId::Object(id) => self.run_object_builtin(id, function, this, args, new_target),
            BuiltinId::Function(id) => {
                self.run_function_builtin(id, function, this, args, new_target)
            }
            BuiltinId::Array(id) => self.run_array_builtin(id, function, this, args, new_target),
            BuiltinId::String(id) => self.run_string_builtin(id, function, this, args, new_target),
            BuiltinId::Math(id) => self.run_math_builtin(id, function, this, args, new_target),
            BuiltinId::Json(id) => self.run_json_builtin(id, function, this, args, new_target),
            BuiltinId::Number(id) => self.run_number_builtin(id, function, this, args, new_target),
            BuiltinId::Boolean(id) => {
                self.run_boolean_builtin(id, function, this, args, new_target)
            }
            BuiltinId::Global(id) => self.run_global_builtin(id, function, this, args, new_target),
        }
    }

    /// The steps of a host function: the `HostFn` with a `Cx`, then the check of what it
    /// returned (decision 6).
    fn run_host_function(
        &mut self,
        function: ObjectRef,
        this: Value,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        let steps = match &self.engine_object(function)?.kind {
            ObjectKind::BuiltinFunction(BuiltinFunction {
                behaviour: Behaviour::Host { steps, .. },
                ..
            }) => steps.downcast_ref::<HostFn<H>>().copied(),
            _ => None,
        };
        let Some(steps) = steps else {
            return Err(self.fault());
        };
        let start = self.scope.len();
        let answer = steps(
            &mut Cx { agent: self },
            HostCall {
                this,
                args,
                new_target,
                callee: function,
            },
        );
        self.scope.truncate(start);
        // A terminated run ignores what the host function returned (decision 10).
        self.running()?;
        match answer {
            Ok(value) => {
                if self.heap.check_value(value).is_err() {
                    return Err(self.fault());
                }
                if new_target.is_some() && !matches!(value, Value::Object(_)) {
                    return Err(self.fault());
                }
                Ok(value)
            }
            Err(Abrupt::Throw(value)) => {
                if self.heap.check_value(value).is_err() {
                    return Err(self.fault());
                }
                Err(Abrupt::Throw(value))
            }
            Err(Abrupt::Terminated(termination)) => Err(self.terminate(termination)),
        }
    }
}
