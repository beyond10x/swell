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
//! A built-in function with slots (`Behaviour::Closure`, `story:promise-constructor-then-and-jobs`)
//! runs the steps of a [`BuiltinId`] with the slots an abstract operation gave it ([`Slots`]):
//! the resolving functions of a promise, the executor of NewPromiseCapability, and the two
//! functions of an `await` (`story:async-functions-and-await`). Its slots are boxed and charged
//! with it, as a host function's boxes are.
//!
//! The element functions of `Promise.all`, `allSettled` and `any` and the closures of
//! `Promise.prototype.finally` (`story:promise-statics-finally-and-aggregate-error`) are built-in
//! functions with slots of their own kind (`Behaviour::PromiseClosure`, [`PromiseSlots`]), so
//! that the matches over [`Slots`] in `ops/promise.rs` and `interp/async_function.rs`, which
//! have no wildcard, stay as they are. An element function holds its [[Index]], its
//! [[AlreadyCalled]] flag (shared by the two functions of an element of `allSettled`) and the
//! record of its call in an `Rc` ([`ElementRecord`]); each function is charged its box, the
//! flag's box and the record's box ("counted in the box of each closure").
//!
//! Two sites of the collector: the kind site [`BuiltinFunction::trace`] and the root site
//! [`trace_roots`] (the activations and the kept stack).

use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;

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
    /// The builtins of `Symbol` (`story:symbol-constructor-and-well-known-symbols`).
    Symbol(crate::builtins::symbol::SymbolBuiltin),
    /// The builtins of the iterator objects
    /// (`story:iterator-protocol-and-array-string-iterators`).
    Iterator(crate::builtins::iterator::IteratorBuiltin),
    /// The builtins of the generator objects
    /// (`story:generator-functions-and-suspended-frames`).
    Generator(crate::builtins::generator::GeneratorBuiltin),
    /// `GeneratorFunction ( ...paramArgs, bodyArg )` (27.6.1.1),
    /// `story:generator-function-constructor`.
    GeneratorFunction,
    /// The builtins of `RegExp` (`story:regexp-objects-exec-and-literals`).
    RegExp(crate::builtins::regexp::RegExpBuiltin),
    /// The builtins of `Promise` and the built-in functions with slots its abstract operations
    /// make (`story:promise-constructor-then-and-jobs`).
    Promise(crate::builtins::promise::PromiseBuiltin),
    /// The two functions an `await` makes (`#sec-await`, steps 3 to 6:
    /// `ReactionType::Fulfill` for onFulfilled, `ReactionType::Reject` for onRejected), with the
    /// slots [`Slots::Await`] (`story:async-functions-and-await`; `interp/async_function.rs`).
    Await(crate::object::promise::ReactionType),
    /// `AsyncFunction ( ...paramArgs, bodyArg )` (`#sec-async-function-constructor-arguments`),
    /// `story:async-function-constructor`.
    AsyncFunction,
    /// The builtins of `%AsyncIteratorPrototype%` and `%AsyncFromSyncIteratorPrototype%` and
    /// the two functions of AsyncFromSyncIteratorContinuation, with the slots
    /// [`Slots::AsyncFromSync`] (`story:for-await-and-async-from-sync-iterator`).
    AsyncIteration(crate::builtins::async_iteration::AsyncIterationBuiltin),
    /// The statics of `Promise` that take an iterable or a callback, `withResolvers`, and the
    /// element functions of the first three
    /// (`story:promise-statics-finally-and-aggregate-error`).
    PromiseStatics(crate::builtins::promise_statics::PromiseStaticsBuiltin),
    /// `AggregateError ( errors, message [ , options ] )` (20.5.7.1.1), whose steps take
    /// `errors` before `message` (`story:promise-statics-finally-and-aggregate-error`).
    AggregateErrorConstructor,
    /// The builtins of `%AsyncGeneratorPrototype%` and the two functions of
    /// AsyncGeneratorAwaitReturn, with the slots [`Slots::Await`]
    /// (`story:async-generators-and-their-request-queue`).
    AsyncGenerator(crate::builtins::async_generator::AsyncGeneratorBuiltin),
}

/// What a built-in function does when called.
pub(crate) enum Behaviour {
    Builtin(BuiltinId),
    /// A host function: its `HostFn<H>` and its host data.
    Host {
        steps: Box<dyn Any>,
        data: Option<HostObject>,
    },
    /// A built-in function with slots, made by an abstract operation (CreateBuiltinFunction with
    /// a list of additional internal slots, `#sec-createbuiltinfunction`): the builtin whose
    /// steps it runs and its slots, in a box charged with the function
    /// ([`Agent::create_builtin_closure`]).
    Closure {
        id: BuiltinId,
        slots: Box<Slots>,
    },
    /// A built-in function with slots of the statics of `Promise` or of `finally`: the builtin
    /// whose steps it runs and its slots, in a box charged with the function
    /// ([`Agent::create_promise_closure`]).
    PromiseClosure {
        id: BuiltinId,
        slots: Box<PromiseSlots>,
    },
}

/// The slots of an element function or of a closure of `finally`: what the Abstract Closure it
/// was made from captured. Matched without a wildcard.
pub(crate) enum PromiseSlots {
    /// A resolve element function of `Promise.all` or `allSettled`, a reject element function of
    /// `allSettled` or `any` (`#sec-performpromiseall` and the two after it): [[Index]],
    /// [[AlreadyCalled]], and the record the functions of one call share.
    Element {
        index: u32,
        already_called: Rc<Cell<bool>>,
        record: Rc<ElementRecord>,
    },
    /// thenFinally or catchFinally of `Promise.prototype.finally`
    /// (`#sec-promise.prototype.finally`, step 6): onFinally and the constructor.
    Finally {
        on_finally: Value,
        constructor: ObjectRef,
    },
    /// valueThunk or thrower of `finally`: the value it returns or throws.
    Thunk { value: Value },
}

/// What the element functions of one call of `Promise.all`, `allSettled` or `any` share:
/// [[Values]] or [[Errors]], an internal Array of the realm of the call that CreateArrayFromList
/// answers as it is once every element is in (no script reaches it before), the
/// [[Capability]] of the combined promise and [[RemainingElements]].
pub(crate) struct ElementRecord {
    pub(crate) values: ObjectRef,
    pub(crate) capability: crate::host::PromiseCapability,
    pub(crate) remaining: Cell<u64>,
}

impl ElementRecord {
    /// The bytes of the record's reference-counted box.
    pub(crate) fn bytes() -> u64 {
        u64::try_from(size_of::<(Cell<usize>, Cell<usize>, ElementRecord)>()).unwrap_or(u64::MAX)
    }
}

impl PromiseSlots {
    /// Every value the slots hold.
    fn trace(&self, mark: &mut Mark) {
        match self {
            PromiseSlots::Element { record, .. } => {
                mark.object(record.values);
                mark.object(record.capability.promise);
                mark.value(record.capability.resolve);
                mark.value(record.capability.reject);
            }
            PromiseSlots::Finally {
                on_finally,
                constructor,
            } => {
                mark.value(*on_finally);
                mark.object(*constructor);
            }
            PromiseSlots::Thunk { value } => mark.value(*value),
        }
    }

    /// The bytes the function with these slots is charged: the box of the slots, and for an
    /// element function the box of its [[AlreadyCalled]] flag and the box of the shared record,
    /// each counted in every function that holds it.
    pub(crate) fn bytes(&self) -> u64 {
        let shared = match self {
            PromiseSlots::Element { .. } => size_of::<(Cell<usize>, Cell<usize>, Cell<bool>)>()
                .saturating_add(usize::try_from(ElementRecord::bytes()).unwrap_or(usize::MAX)),
            PromiseSlots::Finally { .. } | PromiseSlots::Thunk { .. } => 0,
        };
        u64::try_from(size_of::<PromiseSlots>().saturating_add(shared)).unwrap_or(u64::MAX)
    }
}

/// The slots of a built-in function with slots: what the Abstract Closure it was made from
/// captured. Matched without a wildcard.
pub(crate) enum Slots {
    /// A resolving function of CreateResolvingFunctions (`#sec-createresolvingfunctions`): the
    /// promise to settle, and the flag its resolve and reject functions share, set when either
    /// runs.
    Resolving {
        promise: ObjectRef,
        already_resolved: Rc<Cell<bool>>,
    },
    /// executorClosure of NewPromiseCapability (`#sec-newpromisecapability`): the record
    /// resolvingFuncs, its [[Resolve]] and [[Reject]].
    Executor { resolve: Value, reject: Value },
    /// onFulfilled or onRejected of an `await` (`#sec-await`, steps 3 to 6;
    /// `docs/design/engine.md`, decision 3, "`async` functions on suspended frames", item 3):
    /// the activation of the `async` function that awaits, and the number of the suspension the
    /// function was made for. For an `await` of an async generator, and for the two functions
    /// of AsyncGeneratorAwaitReturn (`#sec-asyncgeneratorawaitreturn`), `activation` is the
    /// async generator object and `suspension` the number of its wait ("Async generators on
    /// suspended frames", item 1).
    Await {
        activation: ObjectRef,
        suspension: u64,
    },
    /// The unwrap function or the function that closes the sync iterator of
    /// AsyncFromSyncIteratorContinuation (`#sec-asyncfromsynciteratorcontinuation`;
    /// `builtins/async_iteration.rs`).
    AsyncFromSync(crate::builtins::async_iteration::AsyncFromSyncSlots),
}

impl Slots {
    /// Every value the slots hold.
    fn trace(&self, mark: &mut Mark) {
        match self {
            Slots::Resolving { promise, .. } => mark.object(*promise),
            Slots::Executor { resolve, reject } => {
                mark.value(*resolve);
                mark.value(*reject);
            }
            Slots::Await { activation, .. } => mark.object(*activation),
            Slots::AsyncFromSync(slots) => slots.trace(mark),
        }
    }

    /// The bytes the function with these slots is charged: the box of the slots, and for a
    /// resolving function half of the shared flag's reference-counted box, so that the two
    /// functions of a promise are charged the flag once.
    pub(crate) fn bytes(&self) -> u64 {
        let shared = match self {
            Slots::Resolving { .. } => size_of::<(Cell<usize>, Cell<usize>, Cell<bool>)>() / 2,
            Slots::Executor { .. } | Slots::Await { .. } | Slots::AsyncFromSync(_) => 0,
        };
        u64::try_from(size_of::<Slots>().saturating_add(shared)).unwrap_or(u64::MAX)
    }
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
    /// Kind site of the collector: [[Realm]], [[InitialName]], what a host function's data
    /// lists, and the slots of a built-in function with slots.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/builtin_function.rs BuiltinFunction::trace");
        if let Some(realm) = self.realm {
            mark.push(Item::Realm(realm));
        }
        if let Some(name) = self.initial_name {
            mark.value(Value::String(name));
        }
        match &self.behaviour {
            Behaviour::Builtin(_) | Behaviour::Host { data: None, .. } => {}
            Behaviour::Host {
                data: Some(data), ..
            } => data.trace(mark),
            Behaviour::Closure { slots, .. } => slots.trace(mark),
            Behaviour::PromiseClosure { slots, .. } => slots.trace(mark),
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

    /// A built-in function with slots (CreateBuiltinFunction ( behaviour, length, "", « ... »
    /// ), `#sec-createbuiltinfunction`) in `realm`, which runs the steps of `id` with `slots`:
    /// not a constructor, its `name` the empty String. The box of its slots
    /// ([`Slots::bytes`]) is charged to the memory budget before it is made and recorded in the
    /// function's [`Grown`] as a host function's boxes are, so the sweep that frees it credits
    /// them; over the budget the run ends in `Terminated(MemoryBudget)`.
    ///
    /// [`Grown`]: super::Grown
    pub(crate) fn create_builtin_closure(
        &mut self,
        id: BuiltinId,
        slots: Slots,
        length: u32,
        realm: Handle,
    ) -> Completion<ObjectRef> {
        let bytes = slots.bytes();
        self.create_charged_closure(
            bytes,
            || Behaviour::Closure {
                id,
                slots: Box::new(slots),
            },
            length,
            realm,
        )
    }

    /// As [`Agent::create_builtin_closure`], for the slots of an element function or of a
    /// closure of `finally` ([`PromiseSlots::bytes`]).
    pub(crate) fn create_promise_closure(
        &mut self,
        id: BuiltinId,
        slots: PromiseSlots,
        length: u32,
        realm: Handle,
    ) -> Completion<ObjectRef> {
        let bytes = slots.bytes();
        self.create_charged_closure(
            bytes,
            || Behaviour::PromiseClosure {
                id,
                slots: Box::new(slots),
            },
            length,
            realm,
        )
    }

    /// A built-in function with slots whose box of `bytes` is charged before `behaviour` makes
    /// it and is recorded in the function's [`Grown`].
    ///
    /// [`Grown`]: super::Grown
    fn create_charged_closure(
        &mut self,
        bytes: u64,
        behaviour: impl FnOnce() -> Behaviour,
        length: u32,
        realm: Handle,
    ) -> Completion<ObjectRef> {
        if self.heap.accounting.charge(bytes).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        let made = self.create_builtin_function(behaviour(), length, "", realm, None, false);
        let recorded = made.and_then(|function| {
            self.record_grown(function, bytes)?;
            Ok(function)
        });
        if recorded.is_err() {
            self.heap.accounting.credit(bytes);
        }
        recorded
    }

    /// The slots of the built-in function with slots `function`; any other object is a broken
    /// invariant.
    pub(crate) fn closure_slots(&mut self, function: ObjectRef) -> Completion<&Slots> {
        if !matches!(
            self.engine_object(function)?.kind,
            ObjectKind::BuiltinFunction(BuiltinFunction {
                behaviour: Behaviour::Closure { .. },
                ..
            })
        ) {
            return Err(self.fault());
        }
        match &self.engine_object(function)?.kind {
            ObjectKind::BuiltinFunction(BuiltinFunction {
                behaviour: Behaviour::Closure { slots, .. },
                ..
            }) => Ok(slots.as_ref()),
            _ => Err(Abrupt::Terminated(Termination::EngineFault)),
        }
    }

    /// As [`Agent::closure_slots`], to change.
    pub(crate) fn closure_slots_mut(&mut self, function: ObjectRef) -> Completion<&mut Slots> {
        if !matches!(
            self.engine_object(function)?.kind,
            ObjectKind::BuiltinFunction(BuiltinFunction {
                behaviour: Behaviour::Closure { .. },
                ..
            })
        ) {
            return Err(self.fault());
        }
        match &mut self.engine_object_mut(function)?.kind {
            ObjectKind::BuiltinFunction(BuiltinFunction {
                behaviour: Behaviour::Closure { slots, .. },
                ..
            }) => Ok(slots.as_mut()),
            _ => Err(Abrupt::Terminated(Termination::EngineFault)),
        }
    }

    /// The slots of the element function or closure of `finally` `function`; any other object
    /// is a broken invariant.
    pub(crate) fn promise_closure_slots(
        &mut self,
        function: ObjectRef,
    ) -> Completion<&PromiseSlots> {
        if !matches!(
            self.engine_object(function)?.kind,
            ObjectKind::BuiltinFunction(BuiltinFunction {
                behaviour: Behaviour::PromiseClosure { .. },
                ..
            })
        ) {
            return Err(self.fault());
        }
        match &self.engine_object(function)?.kind {
            ObjectKind::BuiltinFunction(BuiltinFunction {
                behaviour: Behaviour::PromiseClosure { slots, .. },
                ..
            }) => Ok(slots.as_ref()),
            _ => Err(Abrupt::Terminated(Termination::EngineFault)),
        }
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
                    Behaviour::Builtin(id)
                    | Behaviour::Closure { id, .. }
                    | Behaviour::PromiseClosure { id, .. } => Some(id),
                    Behaviour::Host { .. } => None,
                },
            ),
            _ => return Err(self.fault()),
        };
        self.enter_function()?;
        // The activation's copy of the arguments lives while the builtin runs: charged before
        // it is made and credited after the activation is popped.
        let bytes = u64::try_from(size_of_val(args)).unwrap_or(u64::MAX);
        self.charge_bytes(bytes)?;
        #[cfg(test)]
        ACTIVATION_ARGUMENTS_COPIED.with(|copied| {
            let count = u64::try_from(args.len()).unwrap_or(u64::MAX);
            copied.set(copied.get().saturating_add(count));
        });
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
        self.heap.accounting.credit(bytes);
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
            BuiltinId::Symbol(id) => self.run_symbol_builtin(id, function, this, args, new_target),
            BuiltinId::Iterator(id) => {
                self.run_iterator_builtin(id, function, this, args, new_target)
            }
            BuiltinId::RegExp(id) => self.run_regexp_builtin(id, function, this, args, new_target),
            BuiltinId::Generator(id) => {
                self.run_generator_builtin(id, function, this, args, new_target)
            }
            BuiltinId::GeneratorFunction => self.generator_function(function, args, new_target),
            BuiltinId::Promise(id) => {
                self.run_promise_builtin(id, function, this, args, new_target)
            }
            BuiltinId::Await(kind) => {
                let argument = args.first().copied().unwrap_or(Value::Undefined);
                self.await_function(kind, function, argument)
            }
            BuiltinId::AsyncFunction => self.async_function(function, args, new_target),
            BuiltinId::AsyncIteration(id) => {
                self.run_async_iteration_builtin(id, function, this, args)
            }
            BuiltinId::PromiseStatics(id) => {
                self.run_promise_statics_builtin(id, function, this, args)
            }
            BuiltinId::AggregateErrorConstructor => {
                self.aggregate_error_constructor(function, args, new_target)
            }
            BuiltinId::AsyncGenerator(id) => {
                self.run_async_generator_builtin(id, function, this, args)
            }
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

#[cfg(test)]
thread_local! {
    /// The argument values the activations of builtin calls copied, in the counting build of the
    /// tests; a test resets it before it reads it.
    pub(crate) static ACTIVATION_ARGUMENTS_COPIED: std::cell::Cell<u64> =
        const { std::cell::Cell::new(0) };
}

#[cfg(test)]
mod tests {
    use super::ACTIVATION_ARGUMENTS_COPIED;
    use crate::host::agent::Agent;
    use crate::host::hooks::test_host::{NoHooks, agent_with_realm_and, enter, limits};
    use crate::host::outcome::Abrupt;
    use crate::realm::intrinsics::Intrinsic;
    use crate::value::Value;
    use crate::{Realm, Termination};

    /// An agent with no step budget to speak of and a memory budget of what its realm holds
    /// plus 256 MiB, with its realm entered.
    fn roomy() -> (Agent<NoHooks>, Realm, u64) {
        let mut limits = limits();
        limits.steps_per_entry = u64::MAX;
        limits.memory_bytes = 1 << 30;
        let (mut agent, realm) = agent_with_realm_and(limits);
        let roomy_limit = agent.heap.bytes() + (1 << 28);
        agent.heap.accounting.set_limit(roomy_limit);
        enter(&mut agent, &realm);
        (agent, realm, roomy_limit)
    }

    /// `docs/design/engine.md`, "Memory budget" (storage is charged before it is allocated and
    /// credited where its holder drops it), and the execution context of a built-in function's
    /// [[Call]] (BuiltinCallOrConstruct, `#sec-builtincallorconstruct`, step 8, the push of
    /// calleeContext):
    /// `Function.prototype` called with 100,000 Numbers under a budget of the reading after a
    /// collection plus half of their bytes ends in `Terminated(MemoryBudget)` before the
    /// arguments are copied into an activation, with no activation pushed. With the budget
    /// raised the call answers `undefined` and copies each argument once, and after a
    /// collection the reading is the one before.
    #[test]
    fn the_arguments_of_a_builtin_call_are_charged_before_they_are_recorded() {
        const N: u32 = 100_000;
        let (mut agent, realm, roomy_limit) = roomy();
        let realm_handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let prototype = agent
            .intrinsic(realm_handle, Intrinsic::FunctionPrototype)
            .unwrap();
        assert!(agent.collect().is_ok());
        let before = agent.heap_bytes().unwrap();
        let arguments: Vec<Value> = (0..N).map(|i| Value::Number(f64::from(i))).collect();
        let bytes = u64::from(N) * size_of::<Value>() as u64;
        agent.heap.accounting.set_limit(before + bytes / 2);
        ACTIVATION_ARGUMENTS_COPIED.with(|copied| copied.set(0));
        let depth = agent.activations.len();
        let answer = agent.builtin_call(prototype, Value::Undefined, &arguments, None);
        assert!(
            matches!(answer, Err(Abrupt::Terminated(Termination::MemoryBudget))),
            "{answer:?}"
        );
        assert_eq!(
            ACTIVATION_ARGUMENTS_COPIED.with(std::cell::Cell::get),
            0,
            "arguments copied before the memory budget ended the run"
        );
        assert_eq!(agent.activations.len(), depth, "an activation was pushed");
        agent.termination = None;
        agent.heap.accounting.set_limit(roomy_limit);
        let answer = agent.builtin_call(prototype, Value::Undefined, &arguments, None);
        assert!(matches!(answer, Ok(Value::Undefined)), "{answer:?}");
        assert_eq!(
            ACTIVATION_ARGUMENTS_COPIED.with(std::cell::Cell::get),
            u64::from(N)
        );
        assert!(agent.collect().is_ok());
        assert_eq!(agent.heap_bytes().unwrap(), before, "after the call");
    }

    /// `docs/design/engine.md`, "Memory budget" and "Collector" (a credit does not lower the
    /// bytes counted since the last collection, so a transient charge brings a collection
    /// forward): 12,500 calls of `Math.max` with eight arguments each, 100,000 arguments in all,
    /// under a roomy budget run at most one more collection per `MIN_TRIGGER_BYTES` of the
    /// arguments the calls charge than the collections counted on the base of the change, which
    /// charged nothing per call. Not with the feature `collect-every-safe-point`, where every
    /// safe point collects.
    #[cfg(not(feature = "collect-every-safe-point"))]
    #[test]
    fn a_loop_of_builtin_calls_collects_within_its_bound() {
        use crate::heap::{COLLECTIONS, MIN_TRIGGER_BYTES};
        use crate::{Outcome, Parsed, Source};
        const ARGUMENTS: u64 = 100_000;
        const ON_THE_BASE: u64 = 0;
        const SOURCE: &str = "for (var i = 0; i < 12500; i++) Math.max(i, i, i, i, i, i, i, i); i";
        let (mut agent, realm, _) = roomy();
        let Ok(Parsed::Parsed(script)) = agent.parse_script(&realm, Source::Utf8(SOURCE), None)
        else {
            panic!("{SOURCE} did not parse");
        };
        COLLECTIONS.with(|collections| collections.set(0));
        let outcome = agent.evaluate_script(&script);
        let Ok(Outcome::Normal(answer)) = outcome else {
            panic!("{outcome:?}");
        };
        let answer = agent.heap.roots.get(&answer);
        assert!(
            matches!(answer, Ok(Value::Number(n)) if n == 12_500.0),
            "{answer:?}"
        );
        let collections = COLLECTIONS.with(std::cell::Cell::get);
        let charged = ARGUMENTS * size_of::<Value>() as u64;
        let bound = ON_THE_BASE + charged.div_ceil(MIN_TRIGGER_BYTES);
        assert!(
            collections <= bound,
            "{collections} collections, at most {bound}"
        );
    }

    /// The bytes a built-in function with slots is charged for its slots, which
    /// `tests/promises.rs` states as `RESOLVING_BOX_BYTES` and `EXECUTOR_BOX_BYTES`: the box of
    /// the slots, and for a resolving function half of the box of the flag it shares.
    #[test]
    fn the_slots_of_a_closure_are_charged_their_box() {
        use super::Slots;
        let (mut agent, realm, _) = roomy();
        let realm_handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let object = agent
            .intrinsic(realm_handle, Intrinsic::FunctionPrototype)
            .unwrap();
        let resolving = Slots::Resolving {
            promise: object,
            already_resolved: std::rc::Rc::new(std::cell::Cell::new(false)),
        };
        let executor = Slots::Executor {
            resolve: Value::Undefined,
            reject: Value::Undefined,
        };
        let awaiting = Slots::Await {
            activation: object,
            suspension: 1,
        };
        use crate::builtins::async_iteration::AsyncFromSyncSlots;
        let unwrap = Slots::AsyncFromSync(AsyncFromSyncSlots::Unwrap { done: false });
        let close = Slots::AsyncFromSync(AsyncFromSyncSlots::Close { iterator: object });
        let size = size_of::<Slots>();
        assert_eq!(resolving.bytes(), 44, "the slots are {size} bytes");
        assert_eq!(executor.bytes(), 32, "the slots are {size} bytes");
        assert_eq!(awaiting.bytes(), 32, "the slots are {size} bytes");
        assert_eq!(unwrap.bytes(), 32, "the slots are {size} bytes");
        assert_eq!(close.bytes(), 32, "the slots are {size} bytes");
    }

    /// The bytes an element function and a closure of `finally` are charged for their slots,
    /// which `tests/promise_statics.rs` states as `ELEMENT_BOX_BYTES`: the box of the slots, and
    /// for an element function the boxes of its flag and of the record of its call.
    #[test]
    fn the_slots_of_an_element_function_are_charged_their_boxes() {
        use super::{ElementRecord, PromiseSlots};
        let (mut agent, realm, _) = roomy();
        let realm_handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let object = agent
            .intrinsic(realm_handle, Intrinsic::FunctionPrototype)
            .unwrap();
        let element = PromiseSlots::Element {
            index: 0,
            already_called: std::rc::Rc::new(std::cell::Cell::new(false)),
            record: std::rc::Rc::new(ElementRecord {
                values: object,
                capability: crate::host::PromiseCapability {
                    promise: object,
                    resolve: Value::Undefined,
                    reject: Value::Undefined,
                },
                remaining: std::cell::Cell::new(1),
            }),
        };
        let finally = PromiseSlots::Finally {
            on_finally: Value::Undefined,
            constructor: object,
        };
        let thunk = PromiseSlots::Thunk {
            value: Value::Undefined,
        };
        let size = size_of::<PromiseSlots>();
        assert_eq!(ElementRecord::bytes(), 88, "the record's box");
        assert_eq!(element.bytes(), 144, "the slots are {size} bytes");
        assert_eq!(finally.bytes(), 32, "the slots are {size} bytes");
        assert_eq!(thunk.bytes(), 32, "the slots are {size} bytes");
    }
}
