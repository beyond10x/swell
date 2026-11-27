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

use super::dispatch::Flow;
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
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
    /// CreateMethodProperty(prototype, "constructor", F).
    #[inline(never)]
    pub(super) fn make_class_constructor(&mut self, index: u32, derived: bool) -> Completion<Flow> {
        let prototype = self.peek_value(0)?;
        let prototype = self.expect_object(prototype)?;
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
}

#[cfg(test)]
mod tests {
    use crate::host::hooks::test_host::{agent_with_realm, enter};
    use crate::interp::dispatch::Flow;
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
            .make_frame(class, Value::Undefined, &[], Some(object), Some(class))
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
