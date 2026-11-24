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
//! even neighbour. The functions that need an approximation, and `random`, are later stories of
//! slice 4; their rows go after these in the tables, in the order of definition of `%Math%`.
//!
//! `Math.sumPrecise` (`#sec-math.sumprecise`, `story:math-sum-precise`) iterates its argument as
//! `docs/design/engine.md`, decision 3, "Iterators held by a builtin" has it and sums each Number
//! exactly in `swell_num::sum::Sum`, which lives on the frame of the call (decision 6,
//! "`Math.sumPrecise`"): a step per item through `charge_steps` beside the step of the call of
//! `next`, 34 steps once before the sum is rounded, and nothing on the heap.

use crate::builtins::object::argument;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host, Termination};
use crate::object::builtin_function::BuiltinId;
use crate::object::ordinary::OrdinaryData;
use crate::object::property::Property;
use crate::ops::iteration::IteratorRecord;
use crate::realm::intrinsics::Making;
use crate::value::{ObjectRef, Value};
use swell_num::sum::{MAX_ITEMS, Sum};

/// The steps `Math.sumPrecise` charges once before it rounds its sum: one per limb of the
/// subtraction of the two accumulators (decision 6, "`Math.sumPrecise`").
const FINISH_STEPS: u64 = 34;

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
    /// `Math.sumPrecise ( items )` (21.3.2.34).
    SumPrecise,
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
const FUNCTIONS: [(&str, u32, MathBuiltin); 12] = [
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
    // 21.3.2.34, after the functions of fixed arity in the one order of the stories of slice 4
    // (`story:math-sum-precise`).
    ("sumPrecise", 1, MathBuiltin::SumPrecise),
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
            MathBuiltin::SumPrecise => return self.sum_precise(x),
        };
        Ok(Value::Number(number))
    }

    /// `Math.sumPrecise ( items )` (`#sec-math.sumprecise`), out of line so that the 552 bytes
    /// of its `Sum` are on this function's frame and not on that of `run_math_builtin`. The
    /// record's [[Iterator]] and [[NextMethod]] are kept from GetIterator until the loop ends,
    /// on every way out (decision 3, item 1); `items` is held by the activation.
    #[inline(never)]
    fn sum_precise(&mut self, items: Value) -> Completion<Value> {
        // 1. Perform ? RequireObjectCoercible(items) (`#sec-requireobjectcoercible`).
        if matches!(items, Value::Undefined | Value::Null) {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "Math.sumPrecise called on undefined or null",
            ));
        }
        // 2. Let iteratorRecord be ? GetIterator(items, sync).
        let mut record = self.get_iterator(items)?;
        let mark = self.keep(Value::Object(record.iterator));
        self.keep(record.next_method);
        // 3. Let state be minus-zero. 4. Let sum be 0.
        let mut sum = Sum::new();
        let summed = self.sum_items(&mut record, &mut sum);
        self.release(mark);
        summed?;
        // 8 to 12, after the steps of the final pass over the accumulators.
        self.charge_steps(FINISH_STEPS)?;
        Ok(Value::Number(sum.finish()))
    }

    /// Steps 5 to 7 of `#sec-math.sumprecise`: the loop over the iterator into `sum`. Each item
    /// is charged a step through `charge_steps` once IteratorStepValue answered it, beside the
    /// step of the call of `next` (decision 3, item 3); a termination of either ends the loop
    /// with no sum and no IteratorClose (item 4), as does a throw of `next`, of reading `done` or
    /// `value` (IteratorStepValue sets [[Done]]). The caller keeps the record.
    fn sum_items(&mut self, record: &mut IteratorRecord, sum: &mut Sum) -> Completion<()> {
        // 5. Let count be 0.
        let mut count: u64 = 0;
        // 6. Let next be not-started. 7. Repeat, while next is not done,
        loop {
            // 7.a. Set next to ? IteratorStepValue(iteratorRecord). 7.b. If next is not done,
            let Some(next) = self.iterator_step_value(record)? else {
                return Ok(());
            };
            self.charge_steps(1)?;
            // 7.b.i. If count ≥ 2^53 - 1, return ? IteratorClose(iteratorRecord, a RangeError).
            let counted = count.saturating_add(1);
            if check_count(counted).is_err() {
                let error = self.throw_error(
                    ErrorKind::RangeError,
                    "Math.sumPrecise: more than 2^53 - 1 items",
                );
                return Err(self.close_iterator_after(record, error));
            }
            // 7.b.ii. If next is not a Number, return ? IteratorClose(iteratorRecord, a
            // TypeError).
            let Value::Number(n) = next else {
                let error = self.throw_error(
                    ErrorKind::TypeError,
                    "Math.sumPrecise: an item is not a Number",
                );
                return Err(self.close_iterator_after(record, error));
            };
            // 7.b.iii and 7.b.iv: the state and the exact sum. 7.b.v. Set count to count + 1.
            sum.add(n);
            count = counted;
        }
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

/// Step 7.b.i of `Math.sumPrecise` (`#sec-math.sumprecise`) for an item that would be the
/// `n`-th: "If count ≥ 2^53 - 1", with count the items before it, is `n` > `MAX_ITEMS`, the
/// `RangeError` path. Not reachable under a step budget below 2^53.
fn check_count(n: u64) -> Result<(), ()> {
    if n > MAX_ITEMS { Err(()) } else { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::{MAX_ITEMS, check_count};

    /// The count check of `Math.sumPrecise`: the 2^53 - 1-th item is summed, the next is the
    /// `RangeError` path. 2^53 items cannot be run, so the helper the builtin calls is tested.
    #[test]
    fn the_count_check_of_sum_precise_refuses_the_item_after_2_to_the_53_less_1() {
        assert_eq!(MAX_ITEMS, (1 << 53) - 1);
        assert_eq!(check_count(1), Ok(()));
        assert_eq!(check_count(MAX_ITEMS), Ok(()));
        assert_eq!(check_count(MAX_ITEMS + 1), Err(()));
    }
}
