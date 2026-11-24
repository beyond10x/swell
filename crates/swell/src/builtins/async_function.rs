//! AsyncFunction objects (27.7, `#sec-async-function-objects`), written by
//! `story:async-functions-and-await`: `%AsyncFunction.prototype%`
//! (`#sec-async-function-prototype-properties`), the [[Prototype]] of every `async` function
//! and arrow (`object/ecmascript_function.rs`, `instantiate_function`). The steps of an `async`
//! function are `interp/async_function.rs`.
//!
//! `%AsyncFunction%` (`#sec-async-function-constructor`,
//! `#sec-async-function-constructor-properties`) with its `prototype`
//! (`#sec-async-function-constructor-prototype`), and the `constructor` of
//! `%AsyncFunction.prototype%` (`#sec-async-function-prototype-properties-constructor`), written
//! by `story:async-function-constructor`: called or constructed, `AsyncFunction ( ...paramArgs,
//! bodyArg )` (`#sec-async-function-constructor-arguments`) is CreateDynamicFunction of kind
//! `async` (`dynamic_code.rs`). It is no property of the global object: a script reaches it as
//! the `constructor` of the [[Prototype]] of an `async` function.

use crate::dynamic_code::DynamicFunctionKind;
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::object::builtin_function::{Behaviour, BuiltinId};
use crate::object::ordinary::OrdinaryData;
use crate::object::property::Property;
use crate::realm::intrinsics::Making;
use crate::value::{ObjectRef, Value};

impl<H: Host> Agent<H> {
    /// `%AsyncFunction.prototype%` (27.7.3): an ordinary object whose [[Prototype]] is
    /// `%Function.prototype%`, with `[%Symbol.toStringTag%]` "AsyncFunction" { [[Writable]]:
    /// false, [[Enumerable]]: false, [[Configurable]]: true } (27.7.3.2).
    pub(crate) fn create_async_function_prototype(
        &mut self,
        making: Making,
    ) -> Completion<ObjectRef> {
        let prototype =
            self.ordinary_object_create(Some(making.function_prototype), OrdinaryData::Plain)?;
        self.define_to_string_tag(making, prototype, "AsyncFunction")?;
        Ok(prototype)
    }

    /// `%AsyncFunction%`, a constructor whose [[Prototype]] is `function`, `%Function%`, with
    /// "length" 1, "name" "AsyncFunction" and `prototype` holding `function_prototype`,
    /// `%AsyncFunction.prototype%`, { [[Writable]]: false, [[Enumerable]]: false,
    /// [[Configurable]]: false }; and the `constructor` of `%AsyncFunction.prototype%`, holding
    /// it, { [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: true }.
    pub(crate) fn create_async_function_constructor(
        &mut self,
        making: Making,
        function: ObjectRef,
        function_prototype: ObjectRef,
    ) -> Completion<ObjectRef> {
        let constructor = self.create_builtin_function(
            Behaviour::Builtin(BuiltinId::AsyncFunction),
            1,
            "AsyncFunction",
            making.realm,
            Some(function),
            true,
        )?;
        let prototype_key = self.name_key("prototype")?;
        self.add_property(
            constructor,
            prototype_key,
            Property::data(Value::Object(function_prototype), false, false, false),
        )?;
        let constructor_key = self.name_key("constructor")?;
        self.add_property(
            function_prototype,
            constructor_key,
            Property::data(Value::Object(constructor), false, false, true),
        )?;
        Ok(constructor)
    }

    /// `AsyncFunction ( ...paramArgs, bodyArg )` (`#sec-async-function-constructor-arguments`),
    /// called or constructed: the active function `function`, `bodyArg` the last argument or the
    /// empty String, then CreateDynamicFunction(function, NewTarget, async, paramArgs, bodyArg).
    pub(crate) fn async_function(
        &mut self,
        function: ObjectRef,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        let (params, body) = match args.split_last() {
            Some((body, params)) => (params, Some(*body)),
            None => (args, None),
        };
        let new_target = new_target.unwrap_or(function);
        let made =
            self.create_dynamic_function(new_target, DynamicFunctionKind::Async, params, body)?;
        Ok(Value::Object(made))
    }
}
