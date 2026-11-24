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
//! the functions of the constructor (`isFinite`, `isInteger`, `isNaN`, `isSafeInteger`,
//! `parseFloat`, `parseInt`).
//!
//! Slice 4 adds `Number.prototype.toFixed` (`#sec-number.prototype.tofixed`), `toExponential`
//! (`#sec-number.prototype.toexponential`) and `toPrecision`
//! (`#sec-number.prototype.toprecision`), whose digits `swell-num` makes exactly (`format.rs`,
//! `story:number-format`), and `toLocaleString` (`#sec-number.prototype.tolocalestring`), which
//! gives the text of `toString()` of the same Number, with no locale (`docs/design/engine.md`,
//! decision 6, "The text of toLocale*String").

use crate::builtins::object::argument;
use crate::host::agent::Agent;
use crate::host::outcome::{Completion, Termination};
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
    /// `Number.prototype.toFixed ( fractionDigits )` (21.1.3.3).
    ToFixed,
    /// `Number.prototype.toExponential ( fractionDigits )` (21.1.3.2).
    ToExponential,
    /// `Number.prototype.toPrecision ( precision )` (21.1.3.5).
    ToPrecision,
    /// `Number.prototype.toLocaleString ( [ reserved1 [ , reserved2 ] ] )` (21.1.3.4).
    ToLocaleString,
}

/// What one call of `toFixed`, `toExponential` or `toPrecision` charges as its temporary while
/// `swell-num` formats (`docs/design/engine.md`, decision 6, "Big integers in slice 4"): at
/// most six operands of 32 limbs, 1,536 bytes, charged as one temporary of 2,048 bytes before
/// the call and credited when it returns.
const FORMAT_TEMPORARY_BYTES: u64 = 2_048;

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
        // 21.1.3.2, 21.1.3.3 and 21.1.3.5 have one parameter each, so "length" 1; 21.1.3.4 has
        // only optional ones, so 0 (`#sec-ecmascript-standard-built-in-objects`).
        self.define_builtin_method(
            making,
            number_prototype,
            "toFixed",
            1,
            BuiltinId::Number(NumberBuiltin::ToFixed),
        )?;
        self.define_builtin_method(
            making,
            number_prototype,
            "toExponential",
            1,
            BuiltinId::Number(NumberBuiltin::ToExponential),
        )?;
        self.define_builtin_method(
            making,
            number_prototype,
            "toPrecision",
            1,
            BuiltinId::Number(NumberBuiltin::ToPrecision),
        )?;
        self.define_builtin_method(
            making,
            number_prototype,
            "toLocaleString",
            0,
            BuiltinId::Number(NumberBuiltin::ToLocaleString),
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
            NumberBuiltin::ToFixed => self.number_prototype_to_fixed(this, args),
            NumberBuiltin::ToExponential => self.number_prototype_to_exponential(this, args),
            NumberBuiltin::ToPrecision => self.number_prototype_to_precision(this, args),
            NumberBuiltin::ToLocaleString => self.number_prototype_to_locale_string(this),
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

    /// `Number.prototype.toFixed ( fractionDigits )` (`#sec-number.prototype.tofixed`).
    #[inline(never)]
    fn number_prototype_to_fixed(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        // 1. Let x be ? ThisNumberValue(this value).
        let x = self.this_number_value(this, "Number.prototype.toFixed")?;
        // 2. Let f be ? ToIntegerOrInfinity(fractionDigits). 3. (undefined gives 0.)
        let f = self.to_integer_or_infinity(argument(args, 0))?;
        // 4. If f is not an integer (ToIntegerOrInfinity answers one or ±∞), throw a RangeError.
        // 5. If f < 0 or f > 100, throw a RangeError.
        let f = self.count_of_digits(f, 0, "Number.prototype.toFixed")?;
        // 6. If x is not finite, return Number::toString(x, 10).
        if !x.is_finite() {
            return self.number_text(x);
        }
        // 7 to 12, in `swell-num`, which also answers ToString(x) for abs(x) ≥ 10^21 (step 10).
        self.formatted(|| swell_num::format::to_fixed(x, f))
    }

    /// `Number.prototype.toExponential ( fractionDigits )`
    /// (`#sec-number.prototype.toexponential`).
    #[inline(never)]
    fn number_prototype_to_exponential(
        &mut self,
        this: Value,
        args: &[Value],
    ) -> Completion<Value> {
        // 1. Let x be ? ThisNumberValue(this value).
        let x = self.this_number_value(this, "Number.prototype.toExponential")?;
        // 2. Let f be ? ToIntegerOrInfinity(fractionDigits). 3. (undefined gives 0.)
        let digits = argument(args, 0);
        let f = self.to_integer_or_infinity(digits)?;
        // 4. If x is not finite, return Number::toString(x, 10).
        if !x.is_finite() {
            return self.number_text(x);
        }
        // 5. If f is not an integer (ToIntegerOrInfinity answers one or ±∞), throw a RangeError.
        // 6. If f < 0 or f > 100, throw a RangeError.
        let f = self.count_of_digits(f, 0, "Number.prototype.toExponential")?;
        // 7 to 16, in `swell-num`: for an undefined fractionDigits (step 11.b) the shortest
        // digits.
        let f = if matches!(digits, Value::Undefined) {
            None
        } else {
            Some(f)
        };
        self.formatted(|| swell_num::format::to_exponential(x, f))
    }

    /// `Number.prototype.toPrecision ( precision )` (`#sec-number.prototype.toprecision`).
    #[inline(never)]
    fn number_prototype_to_precision(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        // 1. Let x be ? ThisNumberValue(this value).
        let x = self.this_number_value(this, "Number.prototype.toPrecision")?;
        // 2. If precision is undefined, return ! ToString(x).
        let precision = argument(args, 0);
        if matches!(precision, Value::Undefined) {
            return self.number_text(x);
        }
        // 3. Let p be ? ToIntegerOrInfinity(precision).
        let p = self.to_integer_or_infinity(precision)?;
        // 4. If x is not finite, return Number::toString(x, 10).
        if !x.is_finite() {
            return self.number_text(x);
        }
        // 5. If p is not an integer (ToIntegerOrInfinity answers one or ±∞), throw a RangeError.
        // 6. If p < 1 or p > 100, throw a RangeError.
        let p = self.count_of_digits(p, 1, "Number.prototype.toPrecision")?;
        // 7 to 15, in `swell-num`.
        self.formatted(|| swell_num::format::to_precision(x, p))
    }

    /// `Number.prototype.toLocaleString ( [ reserved1 [ , reserved2 ] ] )`
    /// (`#sec-number.prototype.tolocalestring`): the clause leaves the text to the
    /// implementation; it answers what `toString()` answers for the same Number, and the
    /// arguments are read by nothing (`docs/design/engine.md`, decision 6, "The text of
    /// toLocale*String").
    #[inline(never)]
    fn number_prototype_to_locale_string(&mut self, this: Value) -> Completion<Value> {
        let x = self.this_number_value(this, "Number.prototype.toLocaleString")?;
        self.number_text(x)
    }

    /// `Number::toString(x, 10)` as a String, a step charged per code unit of the text before
    /// the String is made, as `toString` charges.
    fn number_text(&mut self, x: f64) -> Completion<Value> {
        let text = swell_num::to_string::number_to_string(x);
        self.charge_steps(u64::try_from(text.len()).unwrap_or(u64::MAX))?;
        Ok(Value::String(self.new_string_str(&text)?))
    }

    /// The count of digits of `method` after ToIntegerOrInfinity: a RangeError when it is not
    /// finite or outside `minimum` to 100.
    fn count_of_digits(&mut self, count: f64, minimum: u32, method: &str) -> Completion<u32> {
        if !count.is_finite() || count < f64::from(minimum) || count > 100.0 {
            return Err(self.throw_error(
                ErrorKind::RangeError,
                &format!("{method} takes from {minimum} to 100 digits"),
            ));
        }
        // An integer from `minimum` to 100 (ToIntegerOrInfinity truncated it), which the cast
        // keeps exactly.
        Ok(count as u32)
    }

    /// The text `format` makes in `swell-num`, as a String. Its temporaries are charged as one
    /// temporary of [`FORMAT_TEMPORARY_BYTES`] before `swell-num` is called (over the budget
    /// the run ends in `Terminated(MemoryBudget)` and nothing is formatted) and credited when it
    /// returns; then one step per code unit of the text is charged before the String is made
    /// (`docs/design/engine.md`, decision 6, "Big integers in slice 4").
    fn formatted(&mut self, format: impl FnOnce() -> String) -> Completion<Value> {
        if self.heap.accounting.charge(FORMAT_TEMPORARY_BYTES).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        let text = format();
        self.heap.accounting.credit(FORMAT_TEMPORARY_BYTES);
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
