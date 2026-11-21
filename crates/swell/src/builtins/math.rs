//! The Math object (ECMA-262 21.3, `#sec-math-object`): `Math.max` (`#sec-math.max`) and
//! `Math[@@toStringTag]` (`#sec-math-%symbol.tostringtag%`). Written by
//! `story:harness-builtins`; the other members are slice 3.

use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{Host, Termination};
use crate::object::builtin_function::BuiltinId;
use crate::object::ordinary::OrdinaryData;
use crate::realm::intrinsics::Making;
use crate::value::{ObjectRef, Value};

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MathBuiltin {
    /// `Math.max ( ...args )` (21.3.2.25).
    Max,
}

impl<H: Host> Agent<H> {
    /// `%Math%` (21.3): an ordinary object whose [[Prototype]] is `%Object.prototype%`, not a
    /// function, with `max` and @@toStringTag "Math" (21.3.1.9).
    pub(crate) fn create_math_object(&mut self, making: Making) -> Completion<ObjectRef> {
        let math =
            self.ordinary_object_create(Some(making.object_prototype), OrdinaryData::Plain)?;
        self.define_builtin_method(making, math, "max", 2, BuiltinId::Math(MathBuiltin::Max))?;
        self.define_to_string_tag(making, math, "Math")?;
        Ok(math)
    }

    /// The steps of the builtins of this module.
    pub(crate) fn run_math_builtin(
        &mut self,
        id: MathBuiltin,
        _function: ObjectRef,
        _this: Value,
        args: &[Value],
        _new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        match id {
            MathBuiltin::Max => self.math_max(args),
        }
    }

    /// `Math.max ( ...args )` (`#sec-math.max`). The arguments are held by this builtin's
    /// activation while ToNumber runs code; a step per argument in each loop.
    fn math_max(&mut self, args: &[Value]) -> Completion<Value> {
        // 1, 2. Let coerced be a new empty List; append ? ToNumber(arg) for each arg. The list
        // lives only for this call; the memory budget is asked before it is made.
        let mut coerced = Vec::new();
        let bytes = u64::try_from(size_of_val(args)).unwrap_or(u64::MAX);
        if self.heap.accounting.check(bytes).is_err()
            || coerced.try_reserve_exact(args.len()).is_err()
        {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        for &arg in args {
            self.charge_steps(1)?;
            coerced.push(self.to_number(arg)?);
        }
        // 3. Let highest be -∞𝔽.
        let mut highest = f64::NEG_INFINITY;
        // 4. For each element number of coerced.
        for number in coerced {
            self.charge_steps(1)?;
            // 4.a. If number is NaN, return NaN.
            if number.is_nan() {
                return Ok(Value::Number(f64::NAN));
            }
            // 4.b. If number is +0𝔽 and highest is -0𝔽, set highest to +0𝔽.
            if number == 0.0
                && number.is_sign_positive()
                && highest == 0.0
                && highest.is_sign_negative()
            {
                highest = 0.0;
            }
            // 4.c. If number > highest, set highest to number.
            if number > highest {
                highest = number;
            }
        }
        // 5. Return highest.
        Ok(Value::Number(highest))
    }
}
