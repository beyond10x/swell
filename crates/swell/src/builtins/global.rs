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
//! `%isFinite%` and `%isNaN%`; and `eval` (19.2.1, `#sec-eval-x`), slice 2's own, the intrinsic
//! `%eval%`, whose steps are `dynamic_code.rs` (`story:eval-and-function-constructor`).
//! `parseFloat` and `parseInt` are slice 4, the URI functions slice 3.
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
    /// `eval ( x )` (19.2.1), slice 2's own.
    Eval,
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

    /// `%eval%` (19.2.1): a built-in function of length 1 (clause 18), not a constructor.
    pub(crate) fn create_eval_function(&mut self, making: Making) -> Completion<ObjectRef> {
        self.create_builtin(making, "eval", 1, BuiltinId::Global(GlobalBuiltin::Eval))
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
        match id {
            // `#sec-eval-x`: 1. Return ? PerformEval(x, false, indirect-eval)
            // (`dynamic_code.rs`).
            GlobalBuiltin::Eval => self.indirect_eval(args),
            GlobalBuiltin::IsFinite => self.number_test(true, args),
            GlobalBuiltin::IsNaN => self.number_test(false, args),
        }
    }

    /// `isFinite` when `finite`, otherwise `isNaN`.
    fn number_test(&mut self, finite: bool, args: &[Value]) -> Completion<Value> {
        // 1. Let number be ? ToNumber(value).
        let number = self.to_number(argument(args, 0))?;
        Ok(Value::Boolean(if finite {
            // `#sec-isfinite-number`: 2. If number is a finite Number, return true. 3. Return
            // false.
            number.is_finite()
        } else {
            // `#sec-isnan-number`: 2. If number is NaN, return true. 3. Return false.
            number.is_nan()
        }))
    }
}
