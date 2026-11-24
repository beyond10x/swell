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
//! even neighbour.
//!
//! The 22 functions whose answer IEEE 754 does not fix (decision 6, "The functions of `Math`
//! that IEEE 754 does not fix"), `acos`, `acosh`, `asin`, `asinh`, `atan`, `atan2`, `atanh`,
//! `cbrt`, `cos`, `cosh`, `exp`, `expm1`, `hypot`, `log`, `log10`, `log1p`, `log2`, `pow`, `sin`,
//! `sinh`, `tan` and `tanh`, convert their arguments here and answer what the function of
//! `swell_num::math` of the same name answers (`exponentiate` for `pow`): it fixes the special
//! values of its clause and computes the rest in plain `f64` arithmetic, so that a script prints
//! the same bytes on every platform (HOST-135). `random` and `sumPrecise` are later stories of
//! slice 4; their rows go after these in the tables, in the order of definition of `%Math%`.

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
    /// `Math.acos ( x )` (21.3.2.2).
    Acos,
    /// `Math.acosh ( x )` (21.3.2.3).
    Acosh,
    /// `Math.asin ( x )` (21.3.2.4).
    Asin,
    /// `Math.asinh ( x )` (21.3.2.5).
    Asinh,
    /// `Math.atan ( x )` (21.3.2.6).
    Atan,
    /// `Math.atan2 ( y, x )` (21.3.2.8).
    Atan2,
    /// `Math.atanh ( x )` (21.3.2.7).
    Atanh,
    /// `Math.cbrt ( x )` (21.3.2.9).
    Cbrt,
    /// `Math.cos ( x )` (21.3.2.12).
    Cos,
    /// `Math.cosh ( x )` (21.3.2.13).
    Cosh,
    /// `Math.exp ( x )` (21.3.2.14).
    Exp,
    /// `Math.expm1 ( x )` (21.3.2.15).
    Expm1,
    /// `Math.hypot ( ...args )` (21.3.2.19).
    Hypot,
    /// `Math.log ( x )` (21.3.2.21).
    Log,
    /// `Math.log10 ( x )` (21.3.2.23).
    Log10,
    /// `Math.log1p ( x )` (21.3.2.22).
    Log1p,
    /// `Math.log2 ( x )` (21.3.2.24).
    Log2,
    /// `Math.pow ( base, exponent )` (21.3.2.27).
    Pow,
    /// `Math.sin ( x )` (21.3.2.31).
    Sin,
    /// `Math.sinh ( x )` (21.3.2.32).
    Sinh,
    /// `Math.tan ( x )` (21.3.2.35).
    Tan,
    /// `Math.tanh ( x )` (21.3.2.36).
    Tanh,
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

/// The functions of `Math` whose answer IEEE 754 does not fix, after those of `FUNCTIONS`, by
/// name, "length" and steps, in the order of definition (21.3.2).
const APPROXIMATED: [(&str, u32, MathBuiltin); 22] = [
    ("acos", 1, MathBuiltin::Acos),
    ("acosh", 1, MathBuiltin::Acosh),
    ("asin", 1, MathBuiltin::Asin),
    ("asinh", 1, MathBuiltin::Asinh),
    ("atan", 1, MathBuiltin::Atan),
    // 21.3.2.8: two parameters, y and x.
    ("atan2", 2, MathBuiltin::Atan2),
    ("atanh", 1, MathBuiltin::Atanh),
    ("cbrt", 1, MathBuiltin::Cbrt),
    ("cos", 1, MathBuiltin::Cos),
    ("cosh", 1, MathBuiltin::Cosh),
    ("exp", 1, MathBuiltin::Exp),
    ("expm1", 1, MathBuiltin::Expm1),
    // 21.3.2.19: "The "length" property of this function is 2𝔽."
    ("hypot", 2, MathBuiltin::Hypot),
    ("log", 1, MathBuiltin::Log),
    ("log10", 1, MathBuiltin::Log10),
    ("log1p", 1, MathBuiltin::Log1p),
    ("log2", 1, MathBuiltin::Log2),
    // 21.3.2.27: two parameters, base and exponent.
    ("pow", 2, MathBuiltin::Pow),
    ("sin", 1, MathBuiltin::Sin),
    ("sinh", 1, MathBuiltin::Sinh),
    ("tan", 1, MathBuiltin::Tan),
    ("tanh", 1, MathBuiltin::Tanh),
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
    /// functions of `FUNCTIONS` and of `APPROXIMATED`, and @@toStringTag "Math" (21.3.1.9).
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
        for (name, length, id) in APPROXIMATED {
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
            // `#sec-math.acos` to `#sec-math.tanh`, each of one argument: 1. Let n be ?
            // ToNumber(x). The steps after it, the special values and the approximation, are
            // the function of `swell_num::math` of the same name.
            MathBuiltin::Acos => return self.math_unary(x, swell_num::math::acos),
            MathBuiltin::Acosh => return self.math_unary(x, swell_num::math::acosh),
            MathBuiltin::Asin => return self.math_unary(x, swell_num::math::asin),
            MathBuiltin::Asinh => return self.math_unary(x, swell_num::math::asinh),
            MathBuiltin::Atan => return self.math_unary(x, swell_num::math::atan),
            MathBuiltin::Atan2 => return self.math_binary(args, swell_num::math::atan2),
            MathBuiltin::Atanh => return self.math_unary(x, swell_num::math::atanh),
            MathBuiltin::Cbrt => return self.math_unary(x, swell_num::math::cbrt),
            MathBuiltin::Cos => return self.math_unary(x, swell_num::math::cos),
            MathBuiltin::Cosh => return self.math_unary(x, swell_num::math::cosh),
            MathBuiltin::Exp => return self.math_unary(x, swell_num::math::exp),
            MathBuiltin::Expm1 => return self.math_unary(x, swell_num::math::expm1),
            MathBuiltin::Hypot => return self.math_hypot(args),
            MathBuiltin::Log => return self.math_unary(x, swell_num::math::log),
            MathBuiltin::Log10 => return self.math_unary(x, swell_num::math::log10),
            MathBuiltin::Log1p => return self.math_unary(x, swell_num::math::log1p),
            MathBuiltin::Log2 => return self.math_unary(x, swell_num::math::log2),
            // `#sec-math.pow`: 3. Return Number::exponentiate(base, exponent).
            MathBuiltin::Pow => return self.math_binary(args, swell_num::math::exponentiate),
            MathBuiltin::Sin => return self.math_unary(x, swell_num::math::sin),
            MathBuiltin::Sinh => return self.math_unary(x, swell_num::math::sinh),
            MathBuiltin::Tan => return self.math_unary(x, swell_num::math::tan),
            MathBuiltin::Tanh => return self.math_unary(x, swell_num::math::tanh),
        };
        Ok(Value::Number(number))
    }

    /// A function of one argument whose answer IEEE 754 does not fix: 1. Let n be ?
    /// ToNumber(x). Then `f`, the function of `swell_num::math` the clause's other steps are,
    /// with no loop and no list.
    fn math_unary(&mut self, x: Value, f: fn(f64) -> f64) -> Completion<Value> {
        let n = self.to_number(x)?;
        Ok(Value::Number(f(n)))
    }

    /// `Math.atan2 ( y, x )` (`#sec-math.atan2`) and `Math.pow ( base, exponent )`
    /// (`#sec-math.pow`): 1. ToNumber of the first argument (y, base). 2. ToNumber of the second
    /// (x, exponent). Then `f` over the two Numbers: `swell_num::math::atan2`, or
    /// `swell_num::math::exponentiate`, which is Number::exponentiate
    /// (`#sec-numeric-types-number-exponentiate`). An argument after the second is not read.
    fn math_binary(&mut self, args: &[Value], f: fn(f64, f64) -> f64) -> Completion<Value> {
        let first = self.to_number(argument(args, 0))?;
        let second = self.to_number(argument(args, 1))?;
        Ok(Value::Number(f(first, second)))
    }

    /// `Math.hypot ( ...args )` (`#sec-math.hypot`): steps 1 and 2 in `Agent::coerce_all` (the
    /// list charged 8 bytes per argument before it is made, a step per argument while ToNumber
    /// runs); then a step per element of the list, charged before `swell_num::math::hypot` runs
    /// over it with the steps after them: an infinity answers +∞, else a NaN answers NaN, else
    /// arguments that are all zero answer +0, else the square root of the sum of squares.
    fn math_hypot(&mut self, args: &[Value]) -> Completion<Value> {
        self.coerce_all(args, |agent, coerced| {
            agent.charge_steps(u64::try_from(coerced.len()).unwrap_or(u64::MAX))?;
            Ok(Value::Number(swell_num::math::hypot(coerced)))
        })
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

#[cfg(test)]
mod tests {
    use crate::host::agent::Agent;
    use crate::host::hooks::test_host::{NoHooks, agent_with_realm_and, enter, limits};
    use crate::realm::intrinsics::Intrinsic;
    use crate::value::{ObjectRef, Value};

    /// The function `Math[name]` of the agent's realm.
    fn math_function(agent: &mut Agent<NoHooks>, math: ObjectRef, name: &str) -> ObjectRef {
        let key = agent.name_key(name).unwrap();
        match agent.get(math, key) {
            Ok(Value::Object(function)) => function,
            other => panic!("Math.{name} is {other:?}"),
        }
    }

    /// `docs/design/engine.md`, "Memory budget": the list of coerced Numbers of `Math.hypot`
    /// (`#sec-math.hypot`, step 1) is charged 8 bytes per argument while the call runs and
    /// credited when it returns. Measured against `Math.atan2` with the same arguments, which
    /// makes no list: the difference of the two high-water marks of the bytes charged is 8,
    /// 800 and 80,000 bytes for 1, 100 and 10,000 arguments, and the two leave the same reading
    /// after the call.
    #[test]
    fn the_list_of_hypot_is_charged_eight_bytes_per_argument() {
        let mut limits = limits();
        limits.steps_per_entry = u64::MAX;
        limits.memory_bytes = 1 << 30;
        let (mut agent, realm) = agent_with_realm_and(limits);
        enter(&mut agent, &realm);
        let realm_handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let math = agent.intrinsic(realm_handle, Intrinsic::Math).unwrap();
        let hypot = math_function(&mut agent, math, "hypot");
        let atan2 = math_function(&mut agent, math, "atan2");
        for count in [1_u64, 100, 10_000] {
            let args = vec![Value::Number(3.0); usize::try_from(count).unwrap()];
            let mut readings = [(0_u64, 0_i128); 2];
            for (reading, function) in readings.iter_mut().zip([hypot, atan2]) {
                let before = agent.heap.accounting.used();
                agent.heap.accounting.reset_high_water();
                let answer = agent.builtin_call(function, Value::Object(math), &args, None);
                assert!(matches!(answer, Ok(Value::Number(_))), "{answer:?}");
                *reading = (
                    agent.heap.accounting.high_water() - before,
                    i128::from(agent.heap.accounting.used()) - i128::from(before),
                );
            }
            let [(hypot_peak, hypot_after), (atan2_peak, atan2_after)] = readings;
            println!(
                "hypot list: {count} argument(s), peak {hypot_peak} bytes against {atan2_peak} \
                 for atan2: {} bytes",
                hypot_peak - atan2_peak
            );
            assert_eq!(hypot_peak - atan2_peak, 8 * count, "{count} argument(s)");
            assert_eq!(hypot_after, atan2_after, "{count} argument(s)");
        }
    }
}
