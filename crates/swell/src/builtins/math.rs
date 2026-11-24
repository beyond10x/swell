//! The Math object (ECMA-262 21.3, `#sec-math-object`). Its value properties
//! (`#sec-value-properties-of-the-math-object`, 21.3.1.1 to 21.3.1.8), `Math[@@toStringTag]`
//! (`#sec-math-%symbol.tostringtag%`), `Math.max` (`#sec-math.max`, `story:harness-builtins`)
//! and the functions whose answer IEEE 754 or integer arithmetic makes exact
//! (`story:math-constants-and-exact-functions`): `abs`, `ceil`, `clz32`, `floor`, `fround`,
//! `imul`, `min`, `round`, `sign`, `sqrt` and `trunc`.
//!
//! Every answer here is the same on every platform (`docs/design/engine.md`, decision 6, "Which
//! `f64` methods an engine crate may call"): `abs`, `floor`, `ceil`, `trunc` and `copysign` are
//! exact (IEEE 754-2019, 5.9 and 5.5.1), `sqrt` is correctly rounded (5.4.1), a comparison is
//! exact, `x as f32` rounds to nearest with ties to even and overflows to the infinities (the
//! Rust Reference, "Numeric cast"), and `clz32` and `imul` work on `u32`. No method the design
//! does not allow is called: the minimum and the maximum are written with comparisons, since
//! the methods of `f64` of those names leave the sign of a zero unspecified, and `Math.round`
//! rounds over `floor`, a comparison and `copysign`, since its ties go toward +∞ and not to the
//! even neighbour. The functions that need an approximation, and `random` and `sumPrecise`, are
//! later stories of slice 4; their rows go after these in the tables, in the order of definition
//! of `%Math%`.

use crate::builtins::object::argument;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{Host, Termination};
use crate::object::builtin_function::BuiltinId;
use crate::object::ordinary::OrdinaryData;
use crate::object::property::Property;
use crate::realm::intrinsics::Making;
use crate::value::{ObjectRef, Value};

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MathBuiltin {
    /// `Math.max ( ...args )` (21.3.2.25).
    Max,
    /// `Math.abs ( x )` (21.3.2.1).
    Abs,
    /// `Math.ceil ( x )` (21.3.2.10).
    Ceil,
    /// `Math.clz32 ( x )` (21.3.2.11).
    Clz32,
    /// `Math.floor ( x )` (21.3.2.16).
    Floor,
    /// `Math.fround ( x )` (21.3.2.17).
    Fround,
    /// `Math.imul ( x, y )` (21.3.2.20).
    Imul,
    /// `Math.min ( ...args )` (21.3.2.26).
    Min,
    /// `Math.round ( x )` (21.3.2.29).
    Round,
    /// `Math.sign ( x )` (21.3.2.30).
    Sign,
    /// `Math.sqrt ( x )` (21.3.2.33).
    Sqrt,
    /// `Math.trunc ( x )` (21.3.2.37).
    Trunc,
}

/// The value properties of `Math` (21.3.1.1 to 21.3.1.8), by name, in the order of definition:
/// each the Number nearest the value the standard states, written with the digits the standard
/// gives.
#[expect(
    clippy::approx_constant,
    clippy::excessive_precision,
    reason = "the digits are the standard's (21.3.1), each naming the Number nearest its value"
)]
const VALUE_PROPERTIES: [(&str, f64); 8] = [
    // 21.3.1.1 Math.E: "approximately 2.7182818284590452354".
    ("E", 2.7182818284590452354),
    // 21.3.1.2 Math.LN10: "approximately 2.302585092994046".
    ("LN10", 2.302585092994046),
    // 21.3.1.3 Math.LN2: "approximately 0.6931471805599453".
    ("LN2", 0.6931471805599453),
    // 21.3.1.4 Math.LOG10E: "approximately 0.4342944819032518".
    ("LOG10E", 0.4342944819032518),
    // 21.3.1.5 Math.LOG2E: "approximately 1.4426950408889634".
    ("LOG2E", 1.4426950408889634),
    // 21.3.1.6 Math.PI: "approximately 3.1415926535897932".
    ("PI", 3.1415926535897932),
    // 21.3.1.7 Math.SQRT1_2: "approximately 0.7071067811865476".
    ("SQRT1_2", 0.7071067811865476),
    // 21.3.1.8 Math.SQRT2: "approximately 1.4142135623730951".
    ("SQRT2", 1.4142135623730951),
];

/// The functions of `Math` after `max`, by name, "length" and steps, in the order of
/// definition (21.3.2).
const FUNCTIONS: [(&str, u32, MathBuiltin); 11] = [
    ("abs", 1, MathBuiltin::Abs),
    ("ceil", 1, MathBuiltin::Ceil),
    ("clz32", 1, MathBuiltin::Clz32),
    ("floor", 1, MathBuiltin::Floor),
    ("fround", 1, MathBuiltin::Fround),
    // 21.3.2.20: two parameters, x and y.
    ("imul", 2, MathBuiltin::Imul),
    // 21.3.2.26: "The "length" property of this function is 2𝔽."
    ("min", 2, MathBuiltin::Min),
    ("round", 1, MathBuiltin::Round),
    ("sign", 1, MathBuiltin::Sign),
    ("sqrt", 1, MathBuiltin::Sqrt),
    ("trunc", 1, MathBuiltin::Trunc),
];

/// Which end of the coerced arguments `Math.max` and `Math.min` answer.
#[derive(Clone, Copy)]
enum Extremum {
    Highest,
    Lowest,
}

impl<H: Host> Agent<H> {
    /// `%Math%` (21.3): an ordinary object whose [[Prototype]] is `%Object.prototype%`, not a
    /// function. Its own properties, in the order of definition: `max`, the value properties
    /// (each { [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: false }), the
    /// functions of `FUNCTIONS`, and @@toStringTag "Math" (21.3.1.9).
    pub(crate) fn create_math_object(&mut self, making: Making) -> Completion<ObjectRef> {
        let math =
            self.ordinary_object_create(Some(making.object_prototype), OrdinaryData::Plain)?;
        self.define_builtin_method(making, math, "max", 2, BuiltinId::Math(MathBuiltin::Max))?;
        for (name, value) in VALUE_PROPERTIES {
            let key = self.name_key(name)?;
            self.add_property(
                math,
                key,
                Property::data(Value::Number(value), false, false, false),
            )?;
        }
        for (name, length, id) in FUNCTIONS {
            self.define_builtin_method(making, math, name, length, BuiltinId::Math(id))?;
        }
        self.define_to_string_tag(making, math, "Math")?;
        Ok(math)
    }

    /// The steps of the builtins of this module. A function of fixed arity makes no loop and no
    /// list: its step is the one of the call.
    pub(crate) fn run_math_builtin(
        &mut self,
        id: MathBuiltin,
        _function: ObjectRef,
        _this: Value,
        args: &[Value],
        _new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        let x = argument(args, 0);
        let number = match id {
            MathBuiltin::Max => return self.extremum(args, Extremum::Highest),
            MathBuiltin::Min => return self.extremum(args, Extremum::Lowest),
            // `#sec-math.abs`: 1. Let n be ? ToNumber(x). 2 to 5: NaN stays NaN, the sign is
            // cleared.
            MathBuiltin::Abs => self.to_number(x)?.abs(),
            // `#sec-math.ceil`: 1. Let n be ? ToNumber(x). 2 to 5: roundToIntegralTowardPositive,
            // which keeps NaN, the zeros and the infinities and gives -0 in (-1, -0).
            MathBuiltin::Ceil => self.to_number(x)?.ceil(),
            // `#sec-math.clz32`: 1. Let n be ? ToUint32(x). 2. The leading zero bits of n.
            MathBuiltin::Clz32 => f64::from(self.to_uint32(x)?.leading_zeros()),
            // `#sec-math.floor`: 1. Let n be ? ToNumber(x). 2 to 5:
            // roundToIntegralTowardNegative, which keeps NaN, the zeros and the infinities and
            // gives +0 in (+0, 1).
            MathBuiltin::Floor => self.to_number(x)?.floor(),
            // `#sec-math.fround`: 1. Let n be ? ToNumber(x). 2 to 6: to binary32 with
            // roundTiesToEven and back; NaN, the zeros and the infinities stay.
            MathBuiltin::Fround => f64::from(self.to_number(x)? as f32),
            MathBuiltin::Imul => {
                // `#sec-math.imul`: 1. Let a be ℝ(? ToUint32(x)). 2. Let b be ℝ(? ToUint32(y)).
                let a = self.to_uint32(x)?;
                let b = self.to_uint32(argument(args, 1))?;
                // 3 to 5. The product modulo 2^32, read as a signed 32-bit integer.
                f64::from(a.wrapping_mul(b).cast_signed())
            }
            MathBuiltin::Round => round(self.to_number(x)?),
            MathBuiltin::Sign => {
                // `#sec-math.sign`: 1. Let n be ? ToNumber(x).
                let n = self.to_number(x)?;
                // 2. If n is one of NaN, +0𝔽, or -0𝔽, return n. 3. If n < -0𝔽, return -1𝔽.
                // 4. Return 1𝔽.
                if n.is_nan() || n == 0.0 {
                    n
                } else if n < 0.0 {
                    -1.0
                } else {
                    1.0
                }
            }
            // `#sec-math.sqrt`: 1. Let n be ? ToNumber(x). 2 to 4: squareRoot, which keeps NaN,
            // the zeros and +∞ and gives NaN below -0.
            MathBuiltin::Sqrt => self.to_number(x)?.sqrt(),
            // `#sec-math.trunc`: 1. Let n be ? ToNumber(x). 2 to 5: roundToIntegralTowardZero,
            // which keeps NaN, the zeros and the infinities and the sign of a zero answer.
            MathBuiltin::Trunc => self.to_number(x)?.trunc(),
        };
        Ok(Value::Number(number))
    }

    /// `Math.max ( ...args )` (`#sec-math.max`) and `Math.min ( ...args )` (`#sec-math.min`):
    /// steps 1 and 2 in `Agent::coerce_all`, then the loop over the coerced list, a step per
    /// element.
    fn extremum(&mut self, args: &[Value], end: Extremum) -> Completion<Value> {
        self.coerce_all(args, |agent, coerced| {
            // 3. Let highest be -∞𝔽 (max); let lowest be +∞𝔽 (min).
            let mut best = match end {
                Extremum::Highest => f64::NEG_INFINITY,
                Extremum::Lowest => f64::INFINITY,
            };
            // 4. For each element number of coerced, do
            for &number in coerced {
                agent.charge_steps(1)?;
                // a. If number is NaN, return NaN.
                if number.is_nan() {
                    return Ok(Value::Number(f64::NAN));
                }
                best = match end {
                    // b. If number is +0𝔽 and highest is -0𝔽, set highest to +0𝔽.
                    // c. If number > highest, set highest to number.
                    Extremum::Highest
                        if number > best
                            || (number == 0.0 && best == 0.0 && number.is_sign_positive()) =>
                    {
                        number
                    }
                    // b. If number is -0𝔽 and lowest is +0𝔽, set lowest to -0𝔽.
                    // c. If number < lowest, set lowest to number.
                    Extremum::Lowest
                        if number < best
                            || (number == 0.0 && best == 0.0 && number.is_sign_negative()) =>
                    {
                        number
                    }
                    _ => best,
                };
            }
            // 5. Return highest (lowest).
            Ok(Value::Number(best))
        })
    }

    /// Steps 1 and 2 of `Math.max`, `Math.min` and `Math.hypot`: "Let coerced be a new empty
    /// List. For each element arg of args, do: Let n be ? ToNumber(arg). Append n to coerced",
    /// then `then` over the list. The list is charged to the memory budget before it is made
    /// (8 bytes per argument) and credited when it is dropped, whatever the answer; a step per
    /// argument while ToNumber runs, which may call script.
    pub(crate) fn coerce_all(
        &mut self,
        args: &[Value],
        then: impl FnOnce(&mut Self, &[f64]) -> Completion<Value>,
    ) -> Completion<Value> {
        let bytes = u64::try_from(size_of::<f64>().saturating_mul(args.len())).unwrap_or(u64::MAX);
        if self.heap.accounting.charge(bytes).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        let answer = self.coerce_all_charged(args, then);
        self.heap.accounting.credit(bytes);
        answer
    }

    /// `Agent::coerce_all` once the list is charged.
    fn coerce_all_charged(
        &mut self,
        args: &[Value],
        then: impl FnOnce(&mut Self, &[f64]) -> Completion<Value>,
    ) -> Completion<Value> {
        let mut coerced = Vec::new();
        if coerced.try_reserve_exact(args.len()).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        for &arg in args {
            self.charge_steps(1)?;
            coerced.push(self.to_number(arg)?);
        }
        then(self, &coerced)
    }
}

/// `Math.round ( x )` (`#sec-math.round`) after ToNumber, by the rule of
/// `docs/design/engine.md`, decision 6, "`Math.round`", over `floor`, a comparison and
/// `copysign`.
fn round(n: f64) -> f64 {
    // 2. If n is not a finite Number or n is an integral Number, return n.
    let r = n.floor();
    if !n.is_finite() || n == r {
        return n;
    }
    // 3 to 5. The integral Number closest to n, ties toward +∞: n - r is exact (the fractional
    // part of a Number is a Number), and r + 1 is exact below 2^52, above which every Number is
    // integral and returned at step 2.
    let y = if n - r >= 0.5 { r + 1.0 } else { r };
    // 3 and 4: a zero answer has the sign of n (+0 in (+0, 0.5), -0 in [-0.5, -0)).
    if y == 0.0 { 0.0_f64.copysign(n) } else { y }
}
