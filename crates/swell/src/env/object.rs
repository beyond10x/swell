//! Object Environment Records (ECMA-262 9.1.1.2, `#sec-object-environment-records`). Filled by
//! `story:interpreter-and-script-entry-points`.
//!
//! The one Object Environment Record of slice 1 is the [[ObjectRecord]] of a global environment,
//! whose [[BindingObject]] is the realm's global object and whose [[IsWithEnvironment]] is false
//! (`with` comes with slice 2). So a record is its binding object, and each concrete method here
//! takes it.

use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};
use crate::object::property::{Descriptor, Key};
use crate::value::{ObjectRef, Value};

impl<H: Host> Agent<H> {
    /// HasBinding (`#sec-object-environment-records-hasbinding-n`) with [[IsWithEnvironment]]
    /// false: HasProperty of the binding object.
    pub(crate) fn object_has_binding(&mut self, binding: ObjectRef, name: Key) -> Completion<bool> {
        self.has_property(binding, name)
    }

    /// CreateMutableBinding (`#sec-object-environment-records-createmutablebinding-n-d`) and
    /// InitializeBinding to `undefined`, as CreateGlobalVarBinding performs them: the property
    /// { [[Value]]: undefined, [[Writable]]: true, [[Enumerable]]: true, [[Configurable]]:
    /// deletable }, then SetMutableBinding with strict false
    /// (`#sec-object-environment-records-initializebinding-n-v`).
    pub(crate) fn object_create_mutable_binding(
        &mut self,
        binding: ObjectRef,
        name: Key,
        deletable: bool,
    ) -> Completion<()> {
        self.define_property_or_throw(
            binding,
            name,
            Descriptor::data(Value::Undefined, true, true, deletable),
        )?;
        self.object_set_mutable_binding(binding, name, Value::Undefined, false)
    }

    /// SetMutableBinding (`#sec-object-environment-records-setmutablebinding-n-v-s`): a binding
    /// that no longer exists is a `ReferenceError` in strict code; then Set with `strict` as its
    /// throw flag.
    pub(crate) fn object_set_mutable_binding(
        &mut self,
        binding: ObjectRef,
        name: Key,
        value: Value,
        strict: bool,
    ) -> Completion<()> {
        let still_exists = self.has_property(binding, name)?;
        if !still_exists && strict {
            return Err(self.throw_error(
                ErrorKind::ReferenceError,
                "the binding was deleted before the assignment",
            ));
        }
        self.set(binding, name, value, strict)?;
        Ok(())
    }

    /// GetBindingValue (`#sec-object-environment-records-getbindingvalue-n-s`): a binding that no
    /// longer exists reads `undefined`, or is a `ReferenceError` in strict code.
    pub(crate) fn object_get_binding_value(
        &mut self,
        binding: ObjectRef,
        name: Key,
        strict: bool,
    ) -> Completion<Value> {
        if !self.has_property(binding, name)? {
            if !strict {
                return Ok(Value::Undefined);
            }
            return Err(self.throw_error(ErrorKind::ReferenceError, "the binding was deleted"));
        }
        self.get(binding, name)
    }

    /// DeleteBinding (`#sec-object-environment-records-deletebinding-n`): [[Delete]] of the
    /// binding object.
    pub(crate) fn object_delete_binding(
        &mut self,
        binding: ObjectRef,
        name: Key,
    ) -> Completion<bool> {
        self.internal_delete(binding, name)
    }
}
