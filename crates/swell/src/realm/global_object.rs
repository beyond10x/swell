//! The global object of a realm (ECMA-262 19, `#sec-global-object`): SetDefaultGlobalBindings
//! (`#sec-setdefaultglobalbindings`) for the properties of clause 19 that this part has, the
//! value properties of 19.1 and the constructors `Error` and the six NativeErrors of 19.3. The
//! global object itself is made by InitializeHostDefinedRealm or given by the host's setup
//! closure (`realm/mod.rs`). Written by `story:object-model-and-realms`.

use super::intrinsics::Intrinsic;
use crate::heap::arena::Handle;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};
use crate::object::property::Descriptor;
use crate::value::Value;

impl<H: Host> Agent<H> {
    /// SetDefaultGlobalBindings (`#sec-setdefaultglobalbindings`): "For each property of the
    /// Global Object specified in clause 19, ... Perform ? DefinePropertyOrThrow(global, name,
    /// desc)".
    pub(crate) fn set_default_global_bindings(&mut self, realm: Handle) -> Completion<()> {
        let Some(global) = self.realm_record(realm)?.global else {
            return Err(self.fault());
        };
        for (name, value) in crate::builtins::global::value_properties() {
            let key = self.name_key(name)?;
            // 19.1: { [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: false }.
            self.define_property_or_throw(
                global,
                key,
                Descriptor::data(value, false, false, false),
            )?;
        }
        for kind in ErrorKind::ALL {
            let constructor = self.intrinsic(realm, Intrinsic::Error(kind))?;
            let key = self.name_key(kind.name())?;
            // Clause 18: { [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]: true }.
            self.define_property_or_throw(
                global,
                key,
                Descriptor::data(Value::Object(constructor), true, false, true),
            )?;
        }
        Ok(())
    }
}
