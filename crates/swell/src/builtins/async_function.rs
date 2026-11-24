//! AsyncFunction objects (27.7, `#sec-async-function-objects`), written by
//! `story:async-functions-and-await`: `%AsyncFunction.prototype%`
//! (`#sec-async-function-prototype-properties`), the [[Prototype]] of every `async` function
//! and arrow (`object/ecmascript_function.rs`, `instantiate_function`). Its `constructor`
//! comes with `%AsyncFunction%` in a later story. The steps of an `async` function are
//! `interp/async_function.rs`.

use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::object::ordinary::OrdinaryData;
use crate::realm::intrinsics::Making;
use crate::value::ObjectRef;

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
}
