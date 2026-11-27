//! The instructions of class definitions (`swell_compiler::bytecode`, after `InitProto`), each in
//! a function of its own that one arm of `interp/dispatch.rs` calls, so the frame of the
//! instruction loop stays as it was (`docs/design/engine.md`, "Classes", items 2 to 4). Written
//! by `story:class-definition-evaluation`.
//!
//! ClassDefinitionEvaluation (`#sec-runtime-semantics-classdefinitionevaluation`) runs as the
//! compiled code of the class (`swell-compiler`, `class.rs`): [`Agent::class_prototype`] makes
//! the prototype, [`Agent::make_class_constructor`] makes F with its box
//! (`object/class_constructor.rs`), `prototype` and `constructor`; the class elements are
//! defined by [`Agent::define_class_method`] and [`Agent::define_class_computed`], not
//! enumerable (`#sec-runtime-semantics-classelementevaluation`). [`Agent::store_initializer`]
//! keeps the initializer of the instance elements in F's box, [`Agent::make_method_instruction`]
//! makes the function of a static element for its call, and [`Agent::init_instance`] runs
//! InitializeInstanceElements (`#sec-initializeinstanceelements`) as a call frame on this loop,
//! no re-entry. `SetStrict` sets the running frame's strictness where the code of a class in
//! non-strict code starts and ends (`#sec-strict-mode-code`; the run loop reads it at every
//! instruction), and [`Agent::set_class_name`] names an anonymous class from the computed key
//! under it before its elements. Every object here is charged before it exists, as every object is
//! (`Agent::ordinary_object_create`, `Agent::instantiate_function`), the box of F with F's
//! record; each instruction is a step of the loop.
//!
//! Derived classes (`story:derived-classes-and-super-call`; "Classes", items 6 to 8):
//! [`Agent::class_heritage`] checks the heritage and makes the prototype from it; a SuperCall is
//! [`Agent::derived_instruction`]'s `GetSuperConstructor`, `SuperCall(n)` (or its spread and
//! kept-arguments forms), whose ECMAScript parent runs in a frame of this loop with the running
//! frame's NewTarget, then `BindThis` into the hidden `this` binding of the constructor in its
//! form; `KeepThis` copies the binding into the frame's `this` before a return, whose rule
//! [`Agent::derived_return`] makes in the caller's realm.

use super::dispatch::Flow;
use super::frame::{Constructed, SlotValue};
use crate::env::{Bound, NameReference};
use crate::host::agent::Agent;
use crate::host::outcome::{Abrupt, Completion};
use crate::host::{ErrorKind, Host};
use crate::object::ObjectKind;
use crate::object::class_constructor::{ClassConstructor, ConstructorKind};
use crate::object::ordinary::OrdinaryData;
use crate::object::property::{Descriptor, Key, Property};
use crate::realm::intrinsics::Intrinsic;
use crate::value::{ObjectRef, Value};
use swell_compiler::bytecode::Instruction;

/// What a class element defines: a method's data property, a getter or a setter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Element {
    Method,
    Getter,
    Setter,
}

impl<H: Host> Agent<H> {
    /// The one arm of the run loop for the instructions of classes: it reads the instruction
    /// again from the running frame (the one before its `pc`, which `Agent::execute_next` has
    /// moved past it) and sends it to the function of its own below. Not inlined and given no
    /// argument, so the arm adds nothing to the frame of the loop (`Agent::execute`), which
    /// stays on the Rust stack at every re-entry. Another instruction never reaches it: a fault
    /// of the agent.
    #[inline(never)]
    pub(super) fn class_instruction(&mut self) -> Completion<Flow> {
        use Instruction as I;
        let instruction = {
            let frame = self.top_frame()?;
            frame
                .pc
                .checked_sub(1)
                .and_then(|at| frame.instructions().get(at).copied())
        };
        let Some(instruction) = instruction else {
            return Err(self.fault());
        };
        match instruction {
            I::ClassPrototype => self.class_prototype(),
            I::ClassHeritage => self.class_heritage(),
            I::MakeClassConstructor { function, derived } => {
                self.make_class_constructor(function, derived)
            }
            I::DefineClassMethod(index) => self.define_class_method(index, Element::Method),
            I::DefineClassGetter(index) => self.define_class_method(index, Element::Getter),
            I::DefineClassSetter(index) => self.define_class_method(index, Element::Setter),
            I::DefineClassComputedMethod => self.define_class_computed(Element::Method),
            I::DefineClassComputedGetter => self.define_class_computed(Element::Getter),
            I::DefineClassComputedSetter => self.define_class_computed(Element::Setter),
            I::StoreInitializer(index) => self.store_initializer(index),
            I::MakeMethod(index) => self.make_method_instruction(index),
            I::InitInstance => self.init_instance(),
            I::SetStrict(strict) => self.set_strict(strict),
            I::SetClassName => self.set_class_name(),
            _ => Err(self.fault()),
        }
    }

    /// `SetStrict`: the running frame's code is strict mode code from here on, or not
    /// (`#sec-strict-mode-code`): where the code of a class starts in non-strict code and where
    /// it ends. A function of its own, so its error path adds nothing to the frame of the class
    /// arm.
    #[inline(never)]
    pub(super) fn set_strict(&mut self, strict: bool) -> Completion<Flow> {
        self.top_frame()?.strict = strict;
        Ok(Flow::Next)
    }

    /// `SetClassName`: SetFunctionName(F, k) (`#sec-setfunctionname`) of F on top with the
    /// property key `k` two entries under it, which `ToPropertyKey` made, before any element
    /// of the class is defined (`#sec-runtime-semantics-classdefinitionevaluation`). F's `name`,
    /// made with F, is defined again, so its own keys stay `length`, `name`, `prototype`.
    #[inline(never)]
    pub(super) fn set_class_name(&mut self) -> Completion<Flow> {
        let function = self.peek_value(0)?;
        let function = self.expect_object(function)?;
        let key = self.peek_value(2)?;
        let key = self.to_property_key(key)?;
        self.set_function_name(function, key, None)?;
        Ok(Flow::Next)
    }

    /// `ClassPrototype`:OrdinaryObjectCreate(protoParent), `%Object.prototype%` of the current
    /// realm for a class without heritage (`#sec-runtime-semantics-classdefinitionevaluation`,
    /// steps 7 and 8).
    #[inline(never)]
    pub(super) fn class_prototype(&mut self) -> Completion<Flow> {
        let parent = self.current_intrinsic(Intrinsic::ObjectPrototype)?;
        let prototype = self.ordinary_object_create(Some(parent), OrdinaryData::Plain)?;
        self.replace(0, Value::Object(prototype))
    }

    /// `MakeClassConstructor { function, derived }` (steps 14 to 19): F made from template
    /// `index` in the LexicalEnvironment, with its name and `length` (OrdinaryFunctionCreate,
    /// SetFunctionName), pushed at once; its box, of kind derived or base (MakeClassConstructor,
    /// `#sec-makeclassconstructor`, and [[ConstructorKind]]); the prototype under it as its
    /// [[HomeObject]]; MakeConstructor(F, false, prototype); then
    /// CreateMethodProperty(prototype, "constructor", F). A derived F's [[Prototype]] is the
    /// constructorParent under the prototype, which `ClassHeritage` left and which leaves the
    /// stack here: `[c, p]` becomes `[p, F]`.
    #[inline(never)]
    pub(super) fn make_class_constructor(&mut self, index: u32, derived: bool) -> Completion<Flow> {
        let prototype = self.peek_value(0)?;
        let prototype = self.expect_object(prototype)?;
        let parent = if derived {
            let parent = self.peek_value(1)?;
            Some(self.expect_object(parent)?)
        } else {
            None
        };
        let (compiled, script, realm, env) = {
            let frame = self.top_frame()?;
            (
                alloc::rc::Rc::clone(&frame.compiled),
                frame.script,
                frame.realm,
                frame.env,
            )
        };
        let function = self.instantiate_function(&compiled, script, realm, index, env)?;
        self.push_value(Value::Object(function))?;
        if let Some(parent) = parent {
            // Step 14 (OrdinaryFunctionCreate with constructorParent): F is new, so the
            // prototype chain of `parent` cannot hold it.
            if !self.ordinary_set_prototype_of(function, Some(parent))? {
                return Err(self.fault());
            }
            self.drop_entries(3)?;
            self.push_value(Value::Object(prototype))?;
            self.push_value(Value::Object(function))?;
        }
        let kind = if derived {
            ConstructorKind::Derived
        } else {
            ConstructorKind::Base
        };
        self.keep_class_box(
            function,
            ClassConstructor {
                kind,
                instance_elements: None,
            },
        )?;
        self.make_method(function, prototype)?;
        let prototype_key = self.name_key("prototype")?;
        self.add_property(
            function,
            prototype_key,
            Property::data(Value::Object(prototype), false, false, false),
        )?;
        let constructor_key = self.name_key("constructor")?;
        self.add_property(
            prototype,
            constructor_key,
            Property::data(Value::Object(function), true, false, true),
        )?;
        Ok(Flow::Next)
    }

    /// `DefineClassMethod(k)`, `DefineClassGetter(k)` and `DefineClassSetter(k)`: the element
    /// `f` on top defined on the home object `h` under it with the key `strings[k]`.
    #[inline(never)]
    pub(super) fn define_class_method(&mut self, index: u32, element: Element) -> Completion<Flow> {
        let home = self.peek_value(1)?;
        let home = self.expect_object(home)?;
        let function = self.peek_value(0)?;
        let function = self.expect_object(function)?;
        let key = self.constant_key(index)?;
        self.define_class_element(home, key, function, element)?;
        self.drop_entries(1)?;
        Ok(Flow::Next)
    }

    /// `DefineClassComputedMethod`, `DefineClassComputedGetter` and `DefineClassComputedSetter`:
    /// SetFunctionName of the element `f` on top with the key `k` under it (and `get` or `set`
    /// for an accessor), then the element defined on the home object `h` under the key.
    #[inline(never)]
    pub(super) fn define_class_computed(&mut self, element: Element) -> Completion<Flow> {
        let home = self.peek_value(2)?;
        let home = self.expect_object(home)?;
        let key = self.peek_value(1)?;
        let key = self.to_property_key(key)?;
        let function = self.peek_value(0)?;
        let function = self.expect_object(function)?;
        let prefix = match element {
            Element::Method => None,
            Element::Getter => Some("get"),
            Element::Setter => Some("set"),
        };
        self.set_function_name(function, key, prefix)?;
        self.define_class_element(home, key, function, element)?;
        self.drop_entries(2)?;
        Ok(Flow::Next)
    }

    /// MakeMethod(f, home) (`#sec-makemethod`), then the property of ClassElementEvaluation:
    /// DefineMethodProperty(home, key, f, false) for a method, a data property { [[Writable]]:
    /// true, [[Enumerable]]: false, [[Configurable]]: true } (`#sec-definemethodproperty`), or
    /// the accessor { [[Enumerable]]: false, [[Configurable]]: true } of a getter or a setter
    /// (`#sec-runtime-semantics-methoddefinitionevaluation`), each by DefinePropertyOrThrow: a
    /// static element keyed `prototype` throws a `TypeError`, F's `prototype` being not
    /// configurable.
    fn define_class_element(
        &mut self,
        home: ObjectRef,
        key: Key,
        function: ObjectRef,
        element: Element,
    ) -> Completion<()> {
        self.make_method(function, home)?;
        let descriptor = match element {
            Element::Method => Descriptor {
                value: Some(Value::Object(function)),
                writable: Some(true),
                enumerable: Some(false),
                configurable: Some(true),
                ..Descriptor::default()
            },
            Element::Getter => Descriptor {
                get: Some(Some(function)),
                enumerable: Some(false),
                configurable: Some(true),
                ..Descriptor::default()
            },
            Element::Setter => Descriptor {
                set: Some(Some(function)),
                enumerable: Some(false),
                configurable: Some(true),
                ..Descriptor::default()
            },
        };
        self.define_property_or_throw(home, key, descriptor)
    }

    /// `StoreInitializer(f)`: the initializer of the instance elements made from template
    /// `index` as a method whose home object is F's [[HomeObject]], the prototype, and stored in
    /// F's box ("Classes", item 4). No allocation lies between its making and its store, so
    /// the box holds it before a collection can run.
    #[inline(never)]
    pub(super) fn store_initializer(&mut self, index: u32) -> Completion<Flow> {
        let class = self.peek_value(0)?;
        let class = self.expect_object(class)?;
        let home = match &self.engine_object(class)?.kind {
            ObjectKind::EcmascriptFunction(f) if f.class.is_some() => f.home_object,
            _ => None,
        };
        let (Some(home), Some(mut boxed)) = (home, self.class_box(class)?) else {
            return Err(self.fault());
        };
        let initializer = self.closure_of(index)?;
        self.make_method(initializer, home)?;
        boxed.instance_elements = Some(initializer);
        self.keep_class_box(class, boxed)?;
        Ok(Flow::Next)
    }

    /// `MakeMethod(f)`: a method of template `index` whose [[HomeObject]] is the value on top,
    /// pushed on it: the function of a run of static fields or of a static block, which the
    /// class-definition code calls with `this` F.
    #[inline(never)]
    pub(super) fn make_method_instruction(&mut self, index: u32) -> Completion<Flow> {
        let home = self.peek_value(0)?;
        let home = self.expect_object(home)?;
        let method = self.closure_of(index)?;
        self.push_value(Value::Object(method))?;
        self.make_method(method, home)?;
        Ok(Flow::Next)
    }

    /// The function object of template `index` in the running frame's LexicalEnvironment, as
    /// `Closure` makes it.
    fn closure_of(&mut self, index: u32) -> Completion<ObjectRef> {
        let (compiled, script, realm, env) = {
            let frame = self.top_frame()?;
            (
                alloc::rc::Rc::clone(&frame.compiled),
                frame.script,
                frame.realm,
                frame.env,
            )
        };
        self.instantiate_function(&compiled, script, realm, index, env)
    }

    /// `InitInstance`: InitializeInstanceElements(o, F) (`#sec-initializeinstanceelements`), F
    /// the running frame's function, o on top. With an initializer in F's box: a call frame of
    /// it with `this` o and no arguments, at the step and the depth check of any call
    /// ([`Agent::push_call_frame`]); its return replaces o by `undefined`. Without one, o is
    /// replaced by `undefined` at once.
    #[inline(never)]
    pub(super) fn init_instance(&mut self) -> Completion<Flow> {
        let function = self.top_frame()?.function;
        let initializer = match function {
            Some(function) => self.class_box(function)?.and_then(|b| b.instance_elements),
            None => None,
        };
        let Some(initializer) = initializer else {
            return self.replace(1, Value::Undefined);
        };
        let this = self.peek_value(0)?;
        self.push_value(Value::Object(initializer))?;
        self.push_call_frame(initializer, this, &[], 0)
    }

    /// The one arm of the run loop for the instructions of derived classes (`GetSuperConstructor`
    /// to `KeepThisEnv`, and `UninitializeEnv`; `docs/design/engine.md`, "Classes", items 6 to
    /// 8): read again from the running frame as [`Agent::class_instruction`] reads its own, so
    /// the arm adds nothing to the frame of the loop. Another instruction is a fault.
    #[inline(never)]
    pub(super) fn derived_instruction(&mut self) -> Completion<Flow> {
        use Instruction as I;
        let instruction = {
            let frame = self.top_frame()?;
            frame
                .pc
                .checked_sub(1)
                .and_then(|at| frame.instructions().get(at).copied())
        };
        let Some(instruction) = instruction else {
            return Err(self.fault());
        };
        match instruction {
            I::GetSuperConstructor => self.get_super_constructor(),
            I::SuperCall(count) => self.super_call(count),
            I::SuperCallSpread => self.super_call_spread(),
            I::SuperCallKept(start) => self.super_call_kept(start),
            I::BindThis(slot) => self.bind_this(Place::Local(slot)),
            I::BindThisEnv { hops, slot } => self.bind_this(Place::Env(hops, slot)),
            I::BindThisName(index) => self.bind_this_by_name(index),
            I::KeepThis(slot) => self.keep_this(Place::Local(slot)),
            I::KeepThisEnv { hops, slot } => self.keep_this(Place::Env(hops, slot)),
            I::UninitializeEnv { hops, slot } => self.uninitialize_env(hops, slot),
            _ => Err(self.fault()),
        }
    }

    /// `GetSuperConstructor`: GetSuperConstructor (`#sec-getsuperconstructor`), the
    /// [[GetPrototypeOf]] of the running frame's function, the derived constructor
    /// GetThisEnvironment answers (an arrow's and eval code's frame holds the function they
    /// stand for); `null` when it has none.
    #[inline(never)]
    fn get_super_constructor(&mut self) -> Completion<Flow> {
        let Some(function) = self.top_frame()?.function else {
            return Err(self.fault());
        };
        let parent = self.get_prototype_of(function)?;
        self.push_value(parent.map_or(Value::Null, Value::Object))?;
        Ok(Flow::Next)
    }

    /// The constructor `c` of a SuperCall, `depth` entries down, checked after the arguments
    /// (SuperCall, step 5: a `TypeError` when it is no constructor), with the running frame's
    /// NewTarget (step 3, an object in every frame that admits a SuperCall).
    fn super_target(&mut self, depth: usize) -> Completion<(ObjectRef, ObjectRef)> {
        let constructor = self.peek_value(depth)?;
        if !self.is_constructor(constructor)? {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "the parent of a derived class is not a constructor",
            ));
        }
        let constructor = self.expect_object(constructor)?;
        let Some(new_target) = self.top_frame()?.new_target else {
            return Err(self.fault());
        };
        Ok((constructor, new_target))
    }

    /// `SuperCall(n)`: Construct(c, «a1 .. an», NewTarget) of `[c, a1 .. an]`. An ECMAScript
    /// constructor runs in a frame of this loop ([`Agent::push_construct_frame`], no
    /// re-entry; a derived parent gets no object); any other constructor is constructed from
    /// Rust as `New(n)` constructs it (a re-entry).
    #[inline(never)]
    fn super_call(&mut self, count: u32) -> Completion<Flow> {
        let count = usize::try_from(count).unwrap_or(usize::MAX);
        let (constructor, new_target) = self.super_target(count)?;
        if self.is_ecmascript_function(constructor)? {
            let args = self.peek_arguments(count)?;
            return self.push_construct_frame(constructor, new_target, &args, count);
        }
        let args = self.charged_arguments(count)?;
        let result = self.construct_value(constructor, &args, Some(new_target));
        let result = self.check_interrupt_after(result)?;
        self.replace(count.saturating_add(1), result)
    }

    /// `SuperCallSpread`: `SuperCall(n)` of `[c, args]` with the elements of the Array `args`.
    #[inline(never)]
    fn super_call_spread(&mut self) -> Completion<Flow> {
        let list = self.peek_value(0)?;
        let (args, bytes) = self.spread_arguments(list)?;
        let made = self.super_call_with(&args, 1);
        drop(args);
        self.heap.accounting.credit(bytes);
        made
    }

    /// `SuperCallKept(n)`: `SuperCall` of `[c]` with the arguments the call kept in the frame's
    /// local slots from slot `n` on, as they are (no iterator; ClassDefinitionEvaluation, step
    /// 14.a), a step per argument; the list is charged while it lives.
    #[inline(never)]
    fn super_call_kept(&mut self, start: u32) -> Completion<Flow> {
        let start = usize::try_from(start).unwrap_or(usize::MAX);
        let count = self.top_frame()?.locals.len().saturating_sub(start);
        self.charge_steps(u64::try_from(count).unwrap_or(u64::MAX))?;
        let mut args = self.new_charged_vec(count)?;
        let copied = self
            .interp
            .frames
            .last()
            .and_then(|frame| frame.locals.get(start..))
            .is_some_and(|kept| {
                kept.iter()
                    .all(|slot| slot.value().is_some_and(|value| args.push(value).is_ok()))
            });
        if !copied {
            return Err(self.fault());
        }
        self.super_call_with(&args, 0)
    }

    /// The construction of `[c, ...]` with `args`, `extra` entries above `c` (the argument
    /// Array of a spread, or none), as `SuperCall(n)` constructs it.
    fn super_call_with(&mut self, args: &[Value], extra: usize) -> Completion<Flow> {
        let (constructor, new_target) = self.super_target(extra)?;
        if self.is_ecmascript_function(constructor)? {
            return self.push_construct_frame(constructor, new_target, args, extra);
        }
        let result = self.construct_value(constructor, args, Some(new_target));
        let result = self.check_interrupt_after(result)?;
        self.replace(extra.saturating_add(1), result)
    }

    /// What slot `place` of the running frame holds: a local slot, or a slot of the record
    /// `hops` out from the LexicalEnvironment.
    fn place_state(&mut self, place: Place) -> Completion<SlotValue> {
        match place {
            Place::Local(slot) => {
                let position = usize::try_from(slot).unwrap_or(usize::MAX);
                let state = self.top_frame()?.locals.get(position).copied();
                state.map_or_else(|| Err(self.fault()), Ok)
            }
            Place::Env(hops, slot) => {
                let env = self.top_frame()?.env;
                self.slot_state(env, hops, slot)
            }
        }
    }

    /// Stores `state` into slot `place` of the running frame.
    fn store_place(&mut self, place: Place, state: SlotValue) -> Completion<()> {
        let stored = match place {
            Place::Local(slot) => {
                let position = usize::try_from(slot).unwrap_or(usize::MAX);
                self.top_frame()?
                    .locals
                    .get_mut(position)
                    .map(|local| *local = state)
            }
            Place::Env(hops, slot) => {
                let env = self.top_frame()?.env;
                let handle = self.environment_at(env, hops)?;
                let index = usize::try_from(slot).unwrap_or(usize::MAX);
                self.heap
                    .environments
                    .get_mut(handle)
                    .ok()
                    .and_then(|record| record.slots.get_mut(index))
                    .map(|binding| *binding = state)
            }
        };
        stored.map_or_else(|| Err(self.fault()), Ok)
    }

    /// `BindThis(n)` and `BindThisEnv { hops, slot }`: BindThisValue (`#sec-bindthisvalue`) of
    /// the value on top, which stays, into the `this` binding at `place`: the `ReferenceError`
    /// of step 2 when it is initialized already, after the parent's constructor ran.
    #[inline(never)]
    fn bind_this(&mut self, place: Place) -> Completion<Flow> {
        let value = self.peek_value(0)?;
        match self.place_state(place)? {
            SlotValue::Uninitialized => self.store_place(place, SlotValue::Value(value))?,
            SlotValue::Value(_) => return Err(self.this_bound_twice()),
            // The `this` binding never holds a Private Name: a logic error.
            SlotValue::PrivateName(_) => return Err(self.fault()),
        }
        Ok(Flow::Next)
    }

    /// `BindThisName(k)`: `BindThis` of the binding `strings[k]` (`this`) that eval code with
    /// inDerivedConstructor finds by name: a named slot of the constructor's function
    /// environment, which a direct `eval` makes named (`swell-compiler`, `resolve.rs`).
    #[inline(never)]
    fn bind_this_by_name(&mut self, index: u32) -> Completion<Flow> {
        let value = self.peek_value(0)?;
        let key = self.constant_key(index)?;
        let (env, realm) = {
            let frame = self.top_frame()?;
            (frame.env, frame.realm)
        };
        let NameReference::Declarative(record) = self.resolve_name(env, realm, key)? else {
            return Err(self.fault());
        };
        let Some(Bound::Slot(slot, _)) = self.bound(record, key)? else {
            return Err(self.fault());
        };
        match self.slot_state(Some(record), 0, slot)? {
            SlotValue::Uninitialized => self.set_slot(Some(record), 0, slot, value)?,
            SlotValue::Value(_) => return Err(self.this_bound_twice()),
            // The `this` binding never holds a Private Name: a logic error.
            SlotValue::PrivateName(_) => return Err(self.fault()),
        }
        Ok(Flow::Next)
    }

    /// The `ReferenceError` of BindThisValue, step 2: `super()` called a second time.
    fn this_bound_twice(&mut self) -> Abrupt {
        self.throw_error(
            ErrorKind::ReferenceError,
            "super() was called twice: this is already initialized",
        )
    }

    /// `KeepThis(n)` and `KeepThisEnv { hops, slot }`: the frame's `this` becomes the value of
    /// the `this` binding at `place` when it is initialized and `undefined` otherwise; nothing
    /// is thrown here (the return rule of [`Agent::derived_return`] reads it).
    #[inline(never)]
    fn keep_this(&mut self, place: Place) -> Completion<Flow> {
        let this = match self.place_state(place)? {
            SlotValue::Value(value) => value,
            SlotValue::Uninitialized => Value::Undefined,
            SlotValue::PrivateName(_) => return Err(self.fault()),
        };
        self.top_frame()?.this = this;
        Ok(Flow::Next)
    }

    /// `UninitializeEnv { hops, slot }`: the slot of the record `hops` out becomes
    /// uninitialized (the `this` binding of a derived constructor in its function environment,
    /// before the parameters are bound).
    #[inline(never)]
    fn uninitialize_env(&mut self, hops: u32, slot: u32) -> Completion<Flow> {
        self.store_place(Place::Env(hops, slot), SlotValue::Uninitialized)?;
        Ok(Flow::Next)
    }

    /// `ClassHeritage`: steps 8.d to 8.h and 10 of ClassDefinitionEvaluation
    /// (`#sec-runtime-semantics-classdefinitionevaluation`) with the heritage's value `h` on
    /// top: `null` gives protoParent `null` and constructorParent `%Function.prototype%` of the
    /// current realm; a value that is no constructor throws a `TypeError`; otherwise
    /// protoParent is Get(h, "prototype"), a `TypeError` when it is neither an object nor
    /// `null`, and constructorParent is `h`. `[h]` becomes `[constructorParent, p]`, `p` the
    /// prototype OrdinaryObjectCreate(protoParent) makes.
    #[inline(never)]
    pub(super) fn class_heritage(&mut self) -> Completion<Flow> {
        let heritage = self.peek_value(0)?;
        let (constructor_parent, proto_parent) = if heritage == Value::Null {
            let function_prototype = self.current_intrinsic(Intrinsic::FunctionPrototype)?;
            (Value::Object(function_prototype), None)
        } else {
            if !self.is_constructor(heritage)? {
                return Err(self.throw_error(
                    ErrorKind::TypeError,
                    "the class heritage is not a constructor",
                ));
            }
            let parent = self.expect_object(heritage)?;
            let key = self.name_key("prototype")?;
            let proto_parent = match self.get(parent, key)? {
                Value::Null => None,
                Value::Object(object) => Some(object),
                _ => {
                    return Err(self.throw_error(
                        ErrorKind::TypeError,
                        "the prototype of the class heritage is neither an object nor null",
                    ));
                }
            };
            (heritage, proto_parent)
        };
        let keep = proto_parent.map(|object| self.keep(Value::Object(object)));
        let prototype = self.ordinary_object_create(proto_parent, OrdinaryData::Plain);
        if let Some(keep) = keep {
            self.release(keep);
        }
        let prototype = prototype?;
        self.replace(1, constructor_parent)?;
        self.push_value(Value::Object(prototype))?;
        Ok(Flow::Next)
    }

    /// What a [[Construct]] of `constructor` with NewTarget `new_target` on this loop gives its
    /// frame: a derived constructor no object (`Constructed::Derived`), a base one the object
    /// OrdinaryCreateFromConstructor(newTarget, "%Object.prototype%") makes. Not inlined, so
    /// the derived path adds nothing to the frame of [`Agent::push_construct_frame`].
    #[inline(never)]
    pub(super) fn construction_of(
        &mut self,
        constructor: ObjectRef,
        new_target: ObjectRef,
    ) -> Completion<Constructed> {
        if self.is_derived_constructor(constructor)? {
            return Ok(Constructed::Derived);
        }
        let this = self.ordinary_create_from_constructor(
            new_target,
            Intrinsic::ObjectPrototype,
            OrdinaryData::Plain,
        )?;
        Ok(Constructed::Base(this))
    }

    /// The error of [[Construct]] steps 11 and 12 for a derived constructor whose code returned
    /// `value`, no object, with no object as its `this` either, made with the caller's realm
    /// current: a `TypeError` for a value other than `undefined`, else the `ReferenceError` of
    /// a `this` never bound (`docs/design/engine.md`, "Classes", item 8).
    #[inline(never)]
    pub(crate) fn derived_return(&mut self, value: Value) -> Abrupt {
        match value {
            Value::Undefined => self.unbound_this_error(),
            _ => self.derived_return_error(),
        }
    }

    /// The `ReferenceError` of a derived constructor that returned `undefined` without a bound
    /// `this` ([[Construct]] step 12, GetThisBinding step 2).
    fn unbound_this_error(&mut self) -> Abrupt {
        self.throw_error(
            ErrorKind::ReferenceError,
            "a derived constructor returned before super() initialized this",
        )
    }

    /// The `TypeError` of a derived constructor that returned a value that is neither an
    /// object nor `undefined` ([[Construct]] step 11.b).
    fn derived_return_error(&mut self) -> Abrupt {
        self.throw_error(
            ErrorKind::TypeError,
            "a derived constructor returned a value that is neither an object nor undefined",
        )
    }
}

/// Where the `this` binding of a derived constructor is, for its instructions: a local slot of
/// the running frame, or slot `slot` of the record `hops` out from its LexicalEnvironment.
#[derive(Clone, Copy, Debug)]
enum Place {
    Local(u32),
    Env(u32, u32),
}

#[cfg(test)]
mod tests {
    use crate::host::hooks::test_host::{agent_with_realm, enter};
    use crate::interp::dispatch::Flow;
    use crate::interp::frame::Constructed;
    use crate::object::class_constructor::ConstructorKind;
    use crate::object::ordinary::OrdinaryData;
    use crate::{Outcome, Parsed, Source, Value};

    /// `InitInstance` in the frame of a class constructor whose box holds no initializer (a
    /// class without instance elements): the object on top is replaced by `undefined` and no
    /// frame is pushed.
    #[test]
    fn init_instance_without_an_initializer_pushes_undefined() {
        let (mut agent, realm) = agent_with_realm();
        let source = "(class A { m() { return 1; } });";
        let script = match agent.parse_script(&realm, Source::Utf8(source), None) {
            Ok(Parsed::Parsed(script)) => script,
            other => panic!("{source:?} did not parse: {other:?}"),
        };
        let root = match agent.evaluate_script(&script) {
            Ok(Outcome::Normal(root)) => root,
            other => panic!("{source:?} answered {other:?}"),
        };
        let Ok(Value::Object(class)) = agent.heap.roots.get(&root) else {
            panic!("the class is no object");
        };
        let boxed = agent.class_box(class).unwrap().unwrap();
        assert_eq!(boxed.kind, ConstructorKind::Base);
        assert!(boxed.instance_elements.is_none());
        enter(&mut agent, &realm);
        let object = agent
            .ordinary_object_create(None, OrdinaryData::Plain)
            .unwrap();
        let _kept = agent.heap.roots.add(Value::Object(object));
        let frame = agent
            .make_frame(
                class,
                Value::Undefined,
                &[],
                Constructed::Base(object),
                Some(class),
            )
            .unwrap();
        agent.push_frame(frame);
        let frames = agent.interp.frames.len();
        agent.push_value(Value::Object(object)).unwrap();
        assert!(matches!(agent.init_instance(), Ok(Flow::Next)));
        assert_eq!(agent.interp.frames.len(), frames);
        assert_eq!(agent.top_frame().unwrap().stack.len(), 1);
        assert!(matches!(agent.peek_value(0), Ok(Value::Undefined)));
        agent.pop_frame().unwrap();
    }
}
