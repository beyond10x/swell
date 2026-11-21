//! Object Environment Records (ECMA-262 9.1.1.2, `#sec-object-environment-records`). Filled by
//! `story:interpreter-and-script-entry-points`.
//!
//! An Object Environment Record is the [[ObjectRecord]] of a global environment, whose
//! [[BindingObject]] is the realm's global object and whose [[IsWithEnvironment]] is false, or
//! the record of a `with` statement (`EnvironmentRecord::binding_object` of `env/mod.rs`), whose
//! [[IsWithEnvironment]] is true. A record is its binding object, and each concrete method here
//! takes it; HasBinding has one method for each value of [[IsWithEnvironment]], and
//! WithBaseObject is answered by the lookup that found the record (`interp/dispatch.rs`).

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

    /// HasBinding (`#sec-object-environment-records-hasbinding-n`) with [[IsWithEnvironment]]
    /// true: HasProperty of the binding object, then a name its @@unscopables object blocks
    /// (ToBoolean of Get of the name) is no binding.
    pub(crate) fn with_has_binding(&mut self, binding: ObjectRef, name: Key) -> Completion<bool> {
        if !self.has_property(binding, name)? {
            return Ok(false);
        }
        let Some(symbols) = self.well_known() else {
            return Err(self.fault());
        };
        let unscopables = self.get(binding, Key::Symbol(symbols.unscopables))?;
        let Value::Object(unscopables) = unscopables else {
            return Ok(true);
        };
        let keep = self.keep(Value::Object(unscopables));
        let blocked = self
            .get(unscopables, name)
            .and_then(|value| self.to_boolean(value));
        self.release(keep);
        Ok(!blocked?)
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
