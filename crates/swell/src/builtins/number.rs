//! Number objects (ECMA-262 21.1, `#sec-number-objects`). `%Number.prototype%`, "itself a
//! Number object; it has a [[NumberData]] internal slot with the value +0𝔽" (21.1.3,
//! `#sec-properties-of-the-number-prototype-object`), whose [[Prototype]] is
//! `%Object.prototype%`, is `story:object-model-and-realms`'s.
//!
//! Slice 2 takes these parts early (`docs/design/engine.md`, section 12, "A part before its
//! slice"; `story:number-boolean-and-global-functions-early`), each complete for what it covers:
//! the `Number` constructor (`#sec-number-constructor-number-value`), the value properties of
//! the constructor (`#sec-properties-of-the-number-constructor`), `Number.prototype.constructor`
//! (`#sec-number.prototype.constructor`), `Number.prototype.valueOf`
//! (`#sec-number.prototype.valueof`) and `Number.prototype.toString`
//! (`#sec-number.prototype.tostring`), with ThisNumberValue (`#sec-thisnumbervalue`) and
//! SnapToInteger (`#sec-snaptointeger`); Number::toString in every radix
//! (`#sec-numeric-types-number-tostring`) is `swell-num`'s. The rest of `Number` is slice 4:
//! `toFixed`, `toExponential`, `toPrecision`, `toLocaleString` and the functions of the
//! constructor (`isFinite`, `isInteger`, `isNaN`, `isSafeInteger`, `parseFloat`, `parseInt`).

use crate::builtins::object::argument;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};
use crate::object::ObjectKind;
use crate::object::builtin_function::BuiltinId;
use crate::object::ordinary::OrdinaryData;
use crate::object::property::Property;
use crate::realm::intrinsics::{Intrinsic, Making};
use crate::value::{ObjectRef, Value};

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NumberBuiltin {
    /// `Number ( value )` (21.1.1.1).
    Constructor,
    /// `Number.prototype.toString ( [ radix ] )` (21.1.3.6).
    ToString,
    /// `Number.prototype.valueOf ( )` (21.1.3.7).
    ValueOf,
}

/// The value properties of the Number constructor (21.1.2.1, 21.1.2.6 to 21.1.2.11 and
/// 21.1.2.14), by name.
const VALUE_PROPERTIES: [(&str, f64); 8] = [
    // 21.1.2.1: "the magnitude of the difference between 1 and the smallest value greater than
    // 1 that is representable as a Number value", 2^-52.
    ("EPSILON", f64::EPSILON),
    // 21.1.2.6: 𝔽(2^53 - 1).
    ("MAX_SAFE_INTEGER", 9_007_199_254_740_991.0),
    // 21.1.2.7: "the largest positive finite Number".
    ("MAX_VALUE", f64::MAX),
    // 21.1.2.8: 𝔽(-(2^53 - 1)).
    ("MIN_SAFE_INTEGER", -9_007_199_254_740_991.0),
    // 21.1.2.9: "the smallest positive value of the Number type", the smallest denormalized
    // number, 2^-1074.
    ("MIN_VALUE", 5e-324),
    // 21.1.2.10.
    ("NaN", f64::NAN),
    // 21.1.2.11.
    ("NEGATIVE_INFINITY", f64::NEG_INFINITY),
    // 21.1.2.14.
    ("POSITIVE_INFINITY", f64::INFINITY),
];

impl<H: Host> Agent<H> {
    /// `%Number.prototype%` (`#sec-properties-of-the-number-prototype-object`).
    pub(crate) fn create_number_prototype(
        &mut self,
        object_prototype: ObjectRef,
    ) -> Completion<ObjectRef> {
        self.ordinary_object_create(Some(object_prototype), OrdinaryData::Number(0.0))
    }

    /// `%Number%` (21.1.1) with its "prototype" (21.1.2.15), its value properties, each
    /// { [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: false }, and the
    /// "constructor", `toString` and `valueOf` of `%Number.prototype%`.
    pub(crate) fn create_number_constructor(
        &mut self,
        making: Making,
        number_prototype: ObjectRef,
    ) -> Completion<ObjectRef> {
        // 21.1.1: "has a "length" property whose value is 1𝔽".
        let constructor = self.create_builtin_constructor(
            making,
            "Number",
            1,
            BuiltinId::Number(NumberBuiltin::Constructor),
            number_prototype,
        )?;
        for (name, value) in VALUE_PROPERTIES {
            let key = self.name_key(name)?;
            self.add_property(
                constructor,
                key,
                Property::data(Value::Number(value), false, false, false),
            )?;
        }
        // 21.1.3.6: "The "length" property of this method is 1𝔽".
        self.define_builtin_method(
            making,
            number_prototype,
            "toString",
            1,
            BuiltinId::Number(NumberBuiltin::ToString),
        )?;
        self.define_builtin_method(
            making,
            number_prototype,
            "valueOf",
            0,
            BuiltinId::Number(NumberBuiltin::ValueOf),
        )?;
        Ok(constructor)
    }

    /// The steps of the builtins of this module.
    pub(crate) fn run_number_builtin(
        &mut self,
        id: NumberBuiltin,
        _function: ObjectRef,
        this: Value,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        match id {
            NumberBuiltin::Constructor => self.number_constructor(args, new_target),
            NumberBuiltin::ToString => self.number_prototype_to_string(this, args),
            // `#sec-number.prototype.valueof`: 1. Return ? ThisNumberValue(this value).
            NumberBuiltin::ValueOf => self
                .this_number_value(this, "Number.prototype.valueOf")
                .map(Value::Number),
        }
    }

    /// `Number ( value )` (`#sec-number-constructor-number-value`).
    fn number_constructor(
        &mut self,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        let number = match args.first() {
            // 1. If value is present, then a. Let primitive be ? ToNumeric(value).
            Some(&value) => match self.to_numeric(value)? {
                // c. Else, let n be primitive.
                Value::Number(number) => number,
                // b. If primitive is a BigInt, let n be 𝔽(ℝ(primitive)). No BigInt value
                // exists before slice 12, so this branch cannot be reached: reaching it is an
                // engine fault.
                _ => return Err(self.fault()),
            },
            // 2. Else, let n be +0𝔽.
            None => 0.0,
        };
        // 3. If NewTarget is undefined, return n.
        let Some(new_target) = new_target else {
            return Ok(Value::Number(number));
        };
        // 4. Let obj be ? OrdinaryCreateFromConstructor(NewTarget, "%Number.prototype%",
        // « [[NumberData]] »). 5. Set obj.[[NumberData]] to n. 6. Return obj. n is a Number, which
        // needs no root while the Getter of "prototype" runs.
        let object = self.ordinary_create_from_constructor(
            new_target,
            Intrinsic::NumberPrototype,
            OrdinaryData::Number(number),
        )?;
        Ok(Value::Object(object))
    }

    /// ThisNumberValue ( arg ) (`#sec-thisnumbervalue`), for the method `method`.
    fn this_number_value(&mut self, value: Value, method: &str) -> Completion<f64> {
        match value {
            // 1. If arg is a Number, return arg.
            Value::Number(number) => return Ok(number),
            // 2. If arg is an Object and arg has a [[NumberData]] internal slot, then
            // return arg.[[NumberData]].
            Value::Object(object) => {
                if let ObjectKind::Ordinary(ordinary) = &self.engine_object(object)?.kind
                    && let OrdinaryData::Number(number) = ordinary.data
                {
                    return Ok(number);
                }
            }
            _ => {}
        }
        // 3. Throw a TypeError exception.
        Err(self.throw_error(
            ErrorKind::TypeError,
            &format!("{method} requires that 'this' be a Number"),
        ))
    }

    /// `Number.prototype.toString ( [ radix ] )` (`#sec-number.prototype.tostring`).
    fn number_prototype_to_string(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        // 1. Let x be ? ThisNumberValue(this value).
        let x = self.this_number_value(this, "Number.prototype.toString")?;
        // 2. If radix is undefined, let radixMV be 10.
        // 3. Else, let radixMV be ? SnapToInteger(radix, truncate, 2, 36).
        let radix = match argument(args, 0) {
            Value::Undefined => 10,
            radix => self.snap_to_integer_truncate(radix, 2, 36)?,
        };
        // 4. Return Number::toString(x, radixMV). radixMV is from 2 to 36, so `swell-num`
        // answers it.
        let Some(text) = swell_num::to_string::number_to_string_radix(x, radix) else {
            return Err(self.fault());
        };
        // The digits are made by a loop that runs for the script: a step per code unit of the
        // result, at most 1,077 (radix 2: "-0.", 1,073 zeros and a 1 for -Number.MIN_VALUE,
        // whose last digit stands for 2^-1074, the place of the last bit of every Number).
        self.charge_steps(u64::try_from(text.len()).unwrap_or(u64::MAX))?;
        Ok(Value::String(self.new_string_str(&text)?))
    }

    /// SnapToInteger ( arg, truncate, minimum, maximum ) (`#sec-snaptointeger`), with both
    /// bounds; they fit in a `u32`, and so does the answer.
    fn snap_to_integer_truncate(
        &mut self,
        arg: Value,
        minimum: u32,
        maximum: u32,
    ) -> Completion<u32> {
        // 1. Let number be ? ToNumber(arg).
        let number = self.to_number(arg)?;
        // 2. If number is one of NaN, +∞𝔽, or -∞𝔽, throw a RangeError exception.
        if !number.is_finite() {
            return Err(
                self.throw_error(ErrorKind::RangeError, "the radix must be a finite number")
            );
        }
        // 3. Let mv be ℝ(number). 4. If nonIntHandling is truncate, set mv to truncate(mv).
        // 5. mv is an integer.
        let mv = number.trunc();
        // 6, 7. If mv < minimum or mv > maximum, throw a RangeError exception.
        if mv < f64::from(minimum) || mv > f64::from(maximum) {
            return Err(self.throw_error(
                ErrorKind::RangeError,
                &format!("the radix must be from {minimum} to {maximum}"),
            ));
        }
        // 8. Return mv: an integer from minimum to maximum, which the cast keeps exactly.
        Ok(mv as u32)
    }
}
