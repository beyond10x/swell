//! The global object's own properties of clause 19 that are not constructors or namespaces.
//!
//! The value properties (19.1, `#sec-value-properties-of-the-global-object`): `Infinity`
//! (19.1.1), `NaN` (19.1.3) and `undefined` (19.1.4), written by
//! `story:object-model-and-realms`. `globalThis` (19.1.2) is slice 3 (`HOST-155`).
//!
//! The function properties (19.2, `#sec-function-properties-of-the-global-object`) that slice 2
//! takes early (`docs/design/engine.md`, section 12, "A part before its slice";
//! `story:number-boolean-and-global-functions-early`): `isFinite` (19.2.2,
//! `#sec-isfinite-number`) and `isNaN` (19.2.3, `#sec-isnan-number`), the intrinsics
//! `%isFinite%` and `%isNaN%`. `eval` is slice 2's own; `parseFloat` and `parseInt` are slice 4,
//! the URI functions slice 3.
//!
//! Each is defined on each global object by SetDefaultGlobalBindings
//! (`realm/global_object.rs`).

use crate::builtins::object::argument;
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::object::builtin_function::BuiltinId;
use crate::realm::intrinsics::Making;
use crate::value::{ObjectRef, Value};

/// The value properties of 19.1 this part defines, by name.
pub(crate) fn value_properties() -> [(&'static str, Value); 3] {
    [
        ("Infinity", Value::Number(f64::INFINITY)),
        ("NaN", Value::Number(f64::NAN)),
        ("undefined", Value::Undefined),
    ]
}

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GlobalBuiltin {
    /// `isFinite ( value )` (19.2.2).
    IsFinite,
    /// `isNaN ( value )` (19.2.3).
    IsNaN,
}

impl<H: Host> Agent<H> {
    /// `%isFinite%` and `%isNaN%`: built-in functions of length 1 (clause 18 and 19.2), not
    /// constructors.
    pub(crate) fn create_global_functions(
        &mut self,
        making: Making,
    ) -> Completion<(ObjectRef, ObjectRef)> {
        let is_finite = self.create_builtin(
            making,
            "isFinite",
            1,
            BuiltinId::Global(GlobalBuiltin::IsFinite),
        )?;
        let is_nan =
            self.create_builtin(making, "isNaN", 1, BuiltinId::Global(GlobalBuiltin::IsNaN))?;
        Ok((is_finite, is_nan))
    }

    /// The steps of the builtins of this module.
    pub(crate) fn run_global_builtin(
        &mut self,
        id: GlobalBuiltin,
        _function: ObjectRef,
        _this: Value,
        args: &[Value],
        _new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        // 1. Let number be ? ToNumber(value).
        let number = self.to_number(argument(args, 0))?;
        Ok(Value::Boolean(match id {
            // `#sec-isfinite-number`: 2. If number is a finite Number, return true. 3. Return
            // false.
            GlobalBuiltin::IsFinite => number.is_finite(),
            // `#sec-isnan-number`: 2. If number is NaN, return true. 3. Return false.
            GlobalBuiltin::IsNaN => number.is_nan(),
        }))
    }
}
