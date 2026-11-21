//! The global object of a realm (ECMA-262 19, `#sec-global-object`): SetDefaultGlobalBindings
//! (`#sec-setdefaultglobalbindings`) for the properties of clause 19 that this part has, the
//! value properties of 19.1, the constructors `Error` and the six NativeErrors of 19.3
//! (`story:object-model-and-realms`), and `Array`, `Object`, `String`, `JSON` and `Math` of 19.3
//! and 19.4 (`story:harness-builtins`), and `isFinite`, `isNaN`, `Boolean` and `Number` of 19.2
//! and 19.3, taken early by slice 2 (`story:number-boolean-and-global-functions-early`), and
//! `eval` and `Function` of 19.2 and 19.3 (`story:eval-and-function-constructor`). The
//! global object itself is made by InitializeHostDefinedRealm or given by the host's setup
//! closure (`realm/mod.rs`).

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
        // The constructor properties `Array`, `Object` and `String` (19.3.2, 19.3.25, 19.3.33)
        // and the other properties `JSON` and `Math` (19.4.2, 19.4.3), written by
        // `story:harness-builtins`; the function properties `isFinite` and `isNaN` (19.2.2,
        // 19.2.3) and the constructor properties `Boolean` and `Number` (19.3.8, 19.3.24),
        // taken early by slice 2 (`story:number-boolean-and-global-functions-early`); the
        // function property `eval` (19.2.1) and the constructor property `Function` (19.3.18) of
        // slice 2 (`story:eval-and-function-constructor`); each
        // { [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]: true } (clause 18).
        for (name, which) in [
            ("Array", Intrinsic::Array),
            ("Object", Intrinsic::Object),
            ("String", Intrinsic::String),
            ("JSON", Intrinsic::Json),
            ("Math", Intrinsic::Math),
            ("isFinite", Intrinsic::IsFinite),
            ("isNaN", Intrinsic::IsNaN),
            ("Boolean", Intrinsic::Boolean),
            ("Number", Intrinsic::Number),
            ("eval", Intrinsic::Eval),
            ("Function", Intrinsic::Function),
        ] {
            let value = self.intrinsic(realm, which)?;
            let key = self.name_key(name)?;
            self.define_property_or_throw(
                global,
                key,
                Descriptor::data(Value::Object(value), true, false, true),
            )?;
        }
        Ok(())
    }
}
