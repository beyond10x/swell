//! Number objects (ECMA-262 21.1, `#sec-number-objects`): this part makes
//! `%Number.prototype%`, "itself a Number object; it has a [[NumberData]] internal slot with the
//! value +0𝔽" (21.1.3), whose [[Prototype]] is `%Object.prototype%`, so that ToObject of a Number
//! has its prototype. The constructor and the methods are slice 4. Written by
//! `story:object-model-and-realms`.

use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::object::ordinary::OrdinaryData;
use crate::value::ObjectRef;

impl<H: Host> Agent<H> {
    /// `%Number.prototype%` (`#sec-properties-of-the-number-prototype-object`).
    pub(crate) fn create_number_prototype(
        &mut self,
        object_prototype: ObjectRef,
    ) -> Completion<ObjectRef> {
        self.ordinary_object_create(Some(object_prototype), OrdinaryData::Number(0.0))
    }
}
