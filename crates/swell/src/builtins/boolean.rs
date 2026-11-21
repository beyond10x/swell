//! Boolean objects (ECMA-262 20.3, `#sec-boolean-objects`): this part makes
//! `%Boolean.prototype%`, "itself a Boolean object; it has a [[BooleanData]] internal slot with
//! the value false" (20.3.3), whose [[Prototype]] is `%Object.prototype%`, so that ToObject of a
//! Boolean has its prototype. The constructor and the methods are slice 3. Written by
//! `story:object-model-and-realms`.

use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::object::ordinary::OrdinaryData;
use crate::value::ObjectRef;

impl<H: Host> Agent<H> {
    /// `%Boolean.prototype%` (`#sec-properties-of-the-boolean-prototype-object`).
    pub(crate) fn create_boolean_prototype(
        &mut self,
        object_prototype: ObjectRef,
    ) -> Completion<ObjectRef> {
        self.ordinary_object_create(Some(object_prototype), OrdinaryData::Boolean(false))
    }
}
