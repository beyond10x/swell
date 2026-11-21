//! Boolean objects (ECMA-262 20.3, `#sec-boolean-objects`). `%Boolean.prototype%`, "itself a
//! Boolean object; it has a [[BooleanData]] internal slot with the value false" (20.3.3,
//! `#sec-properties-of-the-boolean-prototype-object`), whose [[Prototype]] is
//! `%Object.prototype%`, is `story:object-model-and-realms`'s.
//!
//! Slice 2 takes the rest early (`docs/design/engine.md`, section 12, "A part before its
//! slice"; `story:number-boolean-and-global-functions-early`), which is all of 20.3: the
//! `Boolean` constructor (`#sec-boolean-constructor-boolean-value`), `Boolean.prototype`
//! (`#sec-boolean.prototype`), `Boolean.prototype.constructor`
//! (`#sec-boolean.prototype.constructor`), `Boolean.prototype.toString`
//! (`#sec-boolean.prototype.tostring`) and `Boolean.prototype.valueOf`
//! (`#sec-boolean.prototype.valueof`), with ThisBooleanValue (`#sec-thisbooleanvalue`).

use crate::builtins::object::argument;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};
use crate::object::ObjectKind;
use crate::object::builtin_function::BuiltinId;
use crate::object::ordinary::OrdinaryData;
use crate::realm::intrinsics::{Intrinsic, Making};
use crate::value::{ObjectRef, Value};

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BooleanBuiltin {
    /// `Boolean ( value )` (20.3.1.1).
    Constructor,
    /// `Boolean.prototype.toString ( )` (20.3.3.2).
    ToString,
    /// `Boolean.prototype.valueOf ( )` (20.3.3.3).
    ValueOf,
}

impl<H: Host> Agent<H> {
    /// `%Boolean.prototype%` (`#sec-properties-of-the-boolean-prototype-object`).
    pub(crate) fn create_boolean_prototype(
        &mut self,
        object_prototype: ObjectRef,
    ) -> Completion<ObjectRef> {
        self.ordinary_object_create(Some(object_prototype), OrdinaryData::Boolean(false))
    }

    /// `%Boolean%` (20.3.1) with its "prototype" (20.3.2.1, { [[Writable]]: false,
    /// [[Enumerable]]: false, [[Configurable]]: false }) and the "constructor", `toString` and
    /// `valueOf` of `%Boolean.prototype%`.
    pub(crate) fn create_boolean_constructor(
        &mut self,
        making: Making,
        boolean_prototype: ObjectRef,
    ) -> Completion<ObjectRef> {
        // 20.3.1: "has a "length" property whose value is 1𝔽" (clause 18 and 20.3.2).
        let constructor = self.create_builtin_constructor(
            making,
            "Boolean",
            1,
            BuiltinId::Boolean(BooleanBuiltin::Constructor),
            boolean_prototype,
        )?;
        self.define_builtin_method(
            making,
            boolean_prototype,
            "toString",
            0,
            BuiltinId::Boolean(BooleanBuiltin::ToString),
        )?;
        self.define_builtin_method(
            making,
            boolean_prototype,
            "valueOf",
            0,
            BuiltinId::Boolean(BooleanBuiltin::ValueOf),
        )?;
        Ok(constructor)
    }

    /// The steps of the builtins of this module.
    pub(crate) fn run_boolean_builtin(
        &mut self,
        id: BooleanBuiltin,
        _function: ObjectRef,
        this: Value,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        match id {
            BooleanBuiltin::Constructor => self.boolean_constructor(args, new_target),
            // `#sec-boolean.prototype.tostring`: 1. Let bool be ? ThisBooleanValue(this value).
            // 2. If bool is true, return "true". 3. Return "false".
            BooleanBuiltin::ToString => {
                let boolean = self.this_boolean_value(this, "Boolean.prototype.toString")?;
                let text = if boolean { "true" } else { "false" };
                Ok(Value::String(self.new_string_str(text)?))
            }
            // `#sec-boolean.prototype.valueof`: 1. Return ? ThisBooleanValue(this value).
            BooleanBuiltin::ValueOf => self
                .this_boolean_value(this, "Boolean.prototype.valueOf")
                .map(Value::Boolean),
        }
    }

    /// `Boolean ( value )` (`#sec-boolean-constructor-boolean-value`).
    fn boolean_constructor(
        &mut self,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        // 1. Let bool be ToBoolean(value).
        let boolean = self.to_boolean(argument(args, 0))?;
        // 2. If NewTarget is undefined, return bool.
        let Some(new_target) = new_target else {
            return Ok(Value::Boolean(boolean));
        };
        // 3. Let obj be ? OrdinaryCreateFromConstructor(NewTarget, "%Boolean.prototype%",
        // « [[BooleanData]] »). 4. Set obj.[[BooleanData]] to bool. 5. Return obj.
        let object = self.ordinary_create_from_constructor(
            new_target,
            Intrinsic::BooleanPrototype,
            OrdinaryData::Boolean(boolean),
        )?;
        Ok(Value::Object(object))
    }

    /// ThisBooleanValue ( arg ) (`#sec-thisbooleanvalue`), for the method `method`.
    fn this_boolean_value(&mut self, value: Value, method: &str) -> Completion<bool> {
        match value {
            // 1. If arg is a Boolean, return arg.
            Value::Boolean(boolean) => return Ok(boolean),
            // 2. If arg is an Object and arg has a [[BooleanData]] internal slot, then
            // return arg.[[BooleanData]].
            Value::Object(object) => {
                if let ObjectKind::Ordinary(ordinary) = &self.engine_object(object)?.kind
                    && let OrdinaryData::Boolean(boolean) = ordinary.data
                {
                    return Ok(boolean);
                }
            }
            _ => {}
        }
        // 3. Throw a TypeError exception.
        Err(self.throw_error(
            ErrorKind::TypeError,
            &format!("{method} requires that 'this' be a Boolean"),
        ))
    }
}
