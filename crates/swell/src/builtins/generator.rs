//! The generator objects of slice 7 (`#sec-generator-objects`), written by
//! `story:generator-functions-and-suspended-frames`: `%GeneratorFunction.prototype%`
//! (`#sec-properties-of-the-generatorfunction-prototype-object`) with its `prototype`
//! (`#sec-generatorfunction.prototype.prototype`) and `[%Symbol.toStringTag%]`
//! (`#sec-generatorfunction.prototype-%symbol.tostringtag%`); `%GeneratorPrototype%`
//! (`#sec-properties-of-generator-prototype`) with its `constructor`
//! (`#sec-generator.prototype.constructor`), `next` (`#sec-generator.prototype.next`),
//! `return` (`#sec-generator.prototype.return`), `throw` (`#sec-generator.prototype.throw`)
//! and `[%Symbol.toStringTag%]` (`#sec-generator.prototype-%symbol.tostringtag%`); and the
//! abstract operations GeneratorValidate (`#sec-generatorvalidate`), GeneratorResume
//! (`#sec-generatorresume`) and GeneratorResumeAbrupt (`#sec-generatorresumeabrupt`).
//!
//! GeneratorResume and GeneratorResumeAbrupt stop before the run: they take the branches that
//! answer without running the generator's code (a completed generator; an abrupt completion
//! given to one that has not started) and otherwise answer the generator and the completion
//! to resume it with ([`Resume`]). `next`, `return` and `throw` then resume it through
//! `interp/suspend.rs` (`docs/design/engine.md`, decision 3, "Suspended frames", items 4 to
//! 6): the checks of a call, then the frame moved back onto the frame stack
//! ([`Agent::take_generator_frame`] sets the state `executing`), then a run loop of its own,
//! one re-entry. The `constructor` of `%GeneratorFunction.prototype%`, `%GeneratorFunction%`,
//! is the story of the constructor.
//!
//! The this value of `next`, `return` and `throw` is validated with an empty brand:
//! [[GeneratorBrand]] is empty for every generator of this slice.

use swell_compiler::bytecode::{TAG_NORMAL, TAG_RETURN, TAG_THROW};

use crate::host::agent::Agent;
use crate::host::outcome::{Abrupt, Completion};
use crate::host::{ErrorKind, Host};
use crate::interp::suspend::SuspendedFrame;
use crate::object::ObjectKind;
use crate::object::builtin_function::BuiltinId;
use crate::object::generator::GeneratorState;
use crate::object::ordinary::OrdinaryData;
use crate::object::property::Property;
use crate::realm::intrinsics::Making;
use crate::value::{ObjectRef, Value};

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GeneratorBuiltin {
    /// `%GeneratorPrototype%.next ( value )` (27.5.1.2).
    Next,
    /// `%GeneratorPrototype%.return ( value )` (27.5.1.3).
    Return,
    /// `%GeneratorPrototype%.throw ( exception )` (27.5.1.4).
    Throw,
}

/// `%GeneratorFunction.prototype%` and `%GeneratorPrototype%` of a realm.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GeneratorPrototypes {
    pub(crate) function_prototype: ObjectRef,
    pub(crate) prototype: ObjectRef,
}

/// What GeneratorResume and GeneratorResumeAbrupt answer before any code of the generator runs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Resume {
    /// The answer, with no run: the iterator result of a completed generator.
    Answer(Value),
    /// Resume `generator` with `value` and its completion's tag (`TAG_NORMAL`, `TAG_THROW` or
    /// `TAG_RETURN`): the generator is suspended, at its start for `TAG_NORMAL` only.
    Run {
        generator: ObjectRef,
        value: Value,
        tag: u32,
    },
}

impl<H: Host> Agent<H> {
    /// `%GeneratorFunction.prototype%`, an ordinary object whose [[Prototype]] is
    /// `%Function.prototype%`, with `prototype` holding `%GeneratorPrototype%` and
    /// `[%Symbol.toStringTag%]` "GeneratorFunction", each { [[Writable]]: false,
    /// [[Enumerable]]: false, [[Configurable]]: true }; and `%GeneratorPrototype%`, an ordinary
    /// object whose [[Prototype]] is `iterator`, `%Iterator.prototype%`, with `constructor`
    /// holding `%GeneratorFunction.prototype%` (the same attributes), the methods `next`,
    /// `return` and `throw` of "length" 1 (clause 18: { [[Writable]]: true, [[Enumerable]]:
    /// false, [[Configurable]]: true }) and `[%Symbol.toStringTag%]` "Generator".
    pub(crate) fn create_generator_prototypes(
        &mut self,
        making: Making,
        iterator: ObjectRef,
    ) -> Completion<GeneratorPrototypes> {
        let function_prototype =
            self.ordinary_object_create(Some(making.function_prototype), OrdinaryData::Plain)?;
        let prototype = self.ordinary_object_create(Some(iterator), OrdinaryData::Plain)?;
        let prototype_key = self.name_key("prototype")?;
        self.add_property(
            function_prototype,
            prototype_key,
            Property::data(Value::Object(prototype), false, false, true),
        )?;
        self.define_to_string_tag(making, function_prototype, "GeneratorFunction")?;
        let constructor_key = self.name_key("constructor")?;
        self.add_property(
            prototype,
            constructor_key,
            Property::data(Value::Object(function_prototype), false, false, true),
        )?;
        for (name, id) in [
            ("next", GeneratorBuiltin::Next),
            ("return", GeneratorBuiltin::Return),
            ("throw", GeneratorBuiltin::Throw),
        ] {
            self.define_builtin_method(making, prototype, name, 1, BuiltinId::Generator(id))?;
        }
        self.define_to_string_tag(making, prototype, "Generator")?;
        Ok(GeneratorPrototypes {
            function_prototype,
            prototype,
        })
    }

    /// The steps of the builtins of this module: GeneratorResume or GeneratorResumeAbrupt with
    /// the this value and the first argument, then the resumption they answer, run.
    pub(crate) fn run_generator_builtin(
        &mut self,
        id: GeneratorBuiltin,
        _function: ObjectRef,
        this: Value,
        args: &[Value],
        _new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        let value = args.first().copied().unwrap_or(Value::Undefined);
        let resume = match id {
            // `#sec-generator.prototype.next`: 1. Return ? GeneratorResume(this value, value,
            // empty).
            GeneratorBuiltin::Next => self.generator_resume(this, value)?,
            // `#sec-generator.prototype.return`: 1. Let g be the this value. 2. Let C be
            // ReturnCompletion(value). 3. Return ? GeneratorResumeAbrupt(g, C, empty).
            GeneratorBuiltin::Return => self.generator_resume_abrupt(this, value, TAG_RETURN)?,
            // `#sec-generator.prototype.throw`: 1. Let g be the this value. 2. Let C be
            // ThrowCompletion(exception). 3. Return ? GeneratorResumeAbrupt(g, C, empty).
            GeneratorBuiltin::Throw => self.generator_resume_abrupt(this, value, TAG_THROW)?,
        };
        match resume {
            Resume::Answer(answer) => Ok(answer),
            Resume::Run {
                generator,
                value,
                tag,
            } => {
                let resumption =
                    self.resume_frame(|agent| agent.take_generator_frame(generator), value, tag)?;
                self.run_resumed(resumption)
            }
        }
    }

    /// GeneratorValidate ( generator, generatorBrand ) (`#sec-generatorvalidate`) with an empty
    /// brand: the generator object and its [[GeneratorState]]. A value that is no generator
    /// object, and a generator whose state is executing, throw a `TypeError` of the current
    /// realm (the realm of the builtin that validates).
    pub(crate) fn generator_validate(
        &mut self,
        generator: Value,
    ) -> Completion<(ObjectRef, GeneratorState)> {
        // 1. Perform ? RequireInternalSlot(generator, [[GeneratorState]]). 2. Perform ?
        // RequireInternalSlot(generator, [[GeneratorBrand]]). 3. If generator.[[GeneratorBrand]]
        // is not generatorBrand, throw a TypeError exception.
        let found = match generator {
            Value::Object(object) => match &self.engine_object(object)?.kind {
                ObjectKind::Generator(record) => Some((object, record.state)),
                _ => None,
            },
            _ => None,
        };
        match found {
            None => Err(self.throw_error(ErrorKind::TypeError, "not a generator object")),
            // 4. Assert: generator also has a [[GeneratorContext]] internal slot. 5. Let state
            // be generator.[[GeneratorState]]. 6. If state is executing, throw a TypeError
            // exception.
            Some((_, GeneratorState::Executing)) => {
                Err(self.throw_error(ErrorKind::TypeError, "the generator is running"))
            }
            // 7. Return state.
            Some(found) => Ok(found),
        }
    }

    /// GeneratorResume ( generator, value, generatorBrand ) (`#sec-generatorresume`) up to the
    /// run: a completed generator answers CreateIteratorResultObject(undefined, true) (step 2)
    /// with no check; a suspended one is to be resumed with `value`, a normal completion.
    pub(crate) fn generator_resume(
        &mut self,
        generator: Value,
        value: Value,
    ) -> Completion<Resume> {
        // 1. Let state be ? GeneratorValidate(generator, generatorBrand).
        let (generator, state) = self.generator_validate(generator)?;
        // 2. If state is completed, return CreateIteratorResultObject(undefined, true).
        if state == GeneratorState::Completed {
            let result = self.create_iter_result_object(Value::Undefined, true)?;
            return Ok(Resume::Answer(Value::Object(result)));
        }
        // 3. Assert: state is either suspended-start or suspended-yield. Steps 4 to 10, the
        // resumption, are the caller's.
        Ok(Resume::Run {
            generator,
            value,
            tag: TAG_NORMAL,
        })
    }

    /// GeneratorResumeAbrupt ( generator, abruptCompletion, generatorBrand )
    /// (`#sec-generatorresumeabrupt`) up to the run, for a return completion (`TAG_RETURN`) or
    /// a throw completion (`TAG_THROW`) of `value`: a generator that has not started is
    /// completed without running its code, its frame dropped and its charge credited at once
    /// (step 2); a completed one answers `{ value, done: true }` for a return and throws
    /// `value` for a throw (step 3), with no check; a suspended-yield one is to be resumed with
    /// the completion.
    pub(crate) fn generator_resume_abrupt(
        &mut self,
        generator: Value,
        value: Value,
        tag: u32,
    ) -> Completion<Resume> {
        // 1. Let state be ? GeneratorValidate(generator, generatorBrand).
        let (generator, mut state) = self.generator_validate(generator)?;
        // 2. If state is suspended-start, then a. Set generator.[[GeneratorState]] to
        // completed. b. NOTE: Once a generator enters the completed state it never leaves it
        // and its associated execution context is never resumed. Any execution state associated
        // with generator can be discarded at this point. c. Set state to completed.
        if state == GeneratorState::SuspendedStart {
            let context = self.complete_generator(generator)?;
            if let Some(context) = context {
                self.release_suspended_frame(context);
            }
            state = GeneratorState::Completed;
        }
        // 3. If state is completed, then a. If abruptCompletion is a return completion, then
        // i. Return CreateIteratorResultObject(abruptCompletion.[[Value]], true). b. Return
        // ? abruptCompletion.
        if state == GeneratorState::Completed {
            if tag == TAG_RETURN {
                let result = self.create_iter_result_object(value, true)?;
                return Ok(Resume::Answer(Value::Object(result)));
            }
            return Err(Abrupt::Throw(value));
        }
        // 4. Assert: state is suspended-yield. Steps 5 to 11, the resumption, are the caller's.
        Ok(Resume::Run {
            generator,
            value,
            tag,
        })
    }

    /// Set [[GeneratorState]] of `generator` to completed and take its frame out, if it holds
    /// one.
    fn complete_generator(&mut self, generator: ObjectRef) -> Completion<Option<SuspendedFrame>> {
        let taken = match &mut self.engine_object_mut(generator)?.kind {
            ObjectKind::Generator(record) => {
                record.state = GeneratorState::Completed;
                Some(record.context.take())
            }
            _ => None,
        };
        match taken {
            Some(context) => Ok(context),
            None => Err(self.fault()),
        }
    }

    /// GeneratorResume, step 5, and GeneratorResumeAbrupt, step 6, after the checks of a
    /// resumption ("Suspended frames", item 5): [[GeneratorState]] executing, and the frame
    /// the generator holds taken out, for `Agent::resume_frame` to push.
    pub(crate) fn take_generator_frame(
        &mut self,
        generator: ObjectRef,
    ) -> Completion<SuspendedFrame> {
        let taken = match &mut self.engine_object_mut(generator)?.kind {
            ObjectKind::Generator(record) => {
                record.state = GeneratorState::Executing;
                record.context.take()
            }
            _ => None,
        };
        match taken {
            Some(context) => Ok(context),
            None => Err(self.fault()),
        }
    }
}
