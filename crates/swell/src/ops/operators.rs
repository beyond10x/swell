//! The runtime semantics of the ES5 operators (ECMA-262 13): ApplyStringOrNumericBinaryOperator
//! (`#sec-applystringornumericbinaryoperator`), the Number operations of 6.1.6.1
//! (`#sec-numeric-types-number-add` and beside it) that the ES5 operators use, the results of
//! `typeof` (`#sec-typeof-operator`), and string concatenation bounded by `string_length`: a
//! result longer than the bound throws a `RangeError` before it is allocated (decision 2).
//! Equality and the relational comparison are `ops/testing.rs`. Written by
//! `story:object-model-and-realms`.
//!
//! `**` (Number::exponentiate, 6.1.6.1.3, `#sec-numeric-types-number-exponentiate`) is not an
//! ES5 operator: its result is implementation-approximated, and it is the one function
//! `swell_num::math::exponentiate` that `Math.pow` calls too (`story:exponentiation-operator`).

use super::conversion::{number_to_int32, number_to_uint32};
use crate::heap::accounting::ChargedVec;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};
use crate::value::{StringRef, Value};
use swell_num::fmath;

/// A binary operator of ApplyStringOrNumericBinaryOperator: ES5's set and `**`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Exponentiate,
    LeftShift,
    SignedRightShift,
    UnsignedRightShift,
    BitwiseAnd,
    BitwiseXor,
    BitwiseOr,
}

/// The Number operation of `operator` (6.1.6.1, Table 2).
pub(crate) fn number_operation(operator: BinaryOperator, x: f64, y: f64) -> f64 {
    let shift = number_to_uint32(y) % 32;
    match operator {
        // Number::add, ::subtract, ::multiply and ::divide are IEEE 754-2019 operations
        // rounding to nearest, ties to even, which f64 arithmetic is.
        BinaryOperator::Add => x + y,
        BinaryOperator::Subtract => x - y,
        BinaryOperator::Multiply => x * y,
        BinaryOperator::Divide => x / y,
        // Number::remainder truncates like C's fmod, which `fmath::fmod` computes in Rust, with the
        // clause's NaN, infinity and zero cases.
        BinaryOperator::Remainder => fmath::fmod(x, y),
        // Number::exponentiate (6.1.6.1.3, `#sec-numeric-types-number-exponentiate`): the
        // function of `swell-num` that `Math.pow` calls, its special cases in the clause's order.
        BinaryOperator::Exponentiate => swell_num::math::exponentiate(x, y),
        BinaryOperator::LeftShift => f64::from(number_to_int32(x).wrapping_shl(shift)),
        BinaryOperator::SignedRightShift => f64::from(number_to_int32(x) >> shift),
        BinaryOperator::UnsignedRightShift => f64::from(number_to_uint32(x) >> shift),
        BinaryOperator::BitwiseAnd => f64::from(number_to_int32(x) & number_to_int32(y)),
        BinaryOperator::BitwiseXor => f64::from(number_to_int32(x) ^ number_to_int32(y)),
        BinaryOperator::BitwiseOr => f64::from(number_to_int32(x) | number_to_int32(y)),
    }
}

/// Number::unaryMinus (`#sec-numeric-types-number-unaryMinus`).
pub(crate) fn number_unary_minus(x: f64) -> f64 {
    -x
}

/// Number::bitwiseNOT (`#sec-numeric-types-number-bitwiseNOT`).
pub(crate) fn number_bitwise_not(x: f64) -> f64 {
    f64::from(!number_to_int32(x))
}

impl<H: Host> Agent<H> {
    /// ApplyStringOrNumericBinaryOperator (`#sec-applystringornumericbinaryoperator`).
    pub(crate) fn apply_string_or_numeric_binary_operator(
        &mut self,
        left: Value,
        operator: BinaryOperator,
        right: Value,
    ) -> Completion<Value> {
        let (left, right) = if operator == BinaryOperator::Add {
            let keep = self.keep(right);
            let left_primitive = self.to_primitive(left, None);
            self.release(keep);
            let left_primitive = left_primitive?;
            let keep = self.keep(left_primitive);
            let right_primitive = self.to_primitive(right, None);
            self.release(keep);
            let right_primitive = right_primitive?;
            if matches!(left_primitive, Value::String(_))
                || matches!(right_primitive, Value::String(_))
            {
                let keep = self.keep(right_primitive);
                let left_string = self.to_string(left_primitive);
                self.release(keep);
                let left_string = left_string?;
                let keep = self.keep(Value::String(left_string));
                let right_string = self.to_string(right_primitive);
                self.release(keep);
                let right_string = right_string?;
                return Ok(Value::String(self.concatenate(left_string, right_string)?));
            }
            (left_primitive, right_primitive)
        } else {
            (left, right)
        };
        let keep = self.keep(right);
        let left_number = self.to_numeric(left);
        self.release(keep);
        let left_number = left_number?;
        let right_number = self.to_numeric(right)?;
        match (left_number, right_number) {
            (Value::Number(x), Value::Number(y)) => {
                Ok(Value::Number(number_operation(operator, x, y)))
            }
            // No BigInt exists before slice 12.
            _ => Err(self.fault()),
        }
    }

    /// The concatenation of two Strings (the `+` of 13.15.3 step 1.c and of 13.8.1). A result
    /// longer than `string_length` throws a `RangeError` before it is allocated; a step per
    /// code unit copied.
    pub(crate) fn concatenate(
        &mut self,
        left: StringRef,
        right: StringRef,
    ) -> Completion<StringRef> {
        let length = self
            .string_length(left)?
            .saturating_add(self.string_length(right)?);
        let length = u64::try_from(length).unwrap_or(u64::MAX);
        if length > self.limits.string_length {
            return Err(self.throw_error(
                ErrorKind::RangeError,
                "the string would be longer than string_length",
            ));
        }
        self.charge_steps(length)?;
        // One buffer of both Strings' units, charged before it is made; its charge is given
        // back before the String is made.
        let left_length = self.string_length(left)?;
        let right_length = self.string_length(right)?;
        let mut units = ChargedVec::new();
        self.reserve_vec(&mut units, left_length.saturating_add(right_length))?;
        self.extend_units(&mut units, left, 0, left_length)?;
        self.extend_units(&mut units, right, 0, right_length)?;
        let units = units.into_vec();
        self.new_string_value(&units)
    }

    /// The result of `typeof` (`#sec-typeof-operator-runtime-semantics-evaluation`, Table 41).
    pub(crate) fn type_of(&mut self, value: Value) -> Completion<&'static str> {
        Ok(match value {
            Value::Undefined => "undefined",
            Value::Null => "object",
            Value::Boolean(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Symbol(_) => "symbol",
            Value::BigInt(_) => "bigint",
            Value::Object(object) => {
                if self.has_call(object)? {
                    "function"
                } else {
                    "object"
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{BinaryOperator, number_bitwise_not, number_operation, number_unary_minus};
    use crate::host::agent::Agent;
    use crate::host::hooks::test_host::{
        NoHooks, agent_with_realm, agent_with_realm_and, enter, limits,
    };
    use crate::host::outcome::{Abrupt, Completion};
    use crate::host::{Cx, ErrorKind, HostCall};
    use crate::object::ordinary::OrdinaryData;
    use crate::realm::intrinsics::Intrinsic;
    use crate::value::{ObjectRef, Value};
    use alloc::string::String;

    /// `value`, held by a root for the rest of the test, so that a collection at a safe point
    /// (the feature `collect-every-safe-point`) keeps it.
    fn held(agent: &mut Agent<NoHooks>, value: Value) -> Value {
        core::mem::forget(agent.heap.roots.add(value));
        value
    }

    fn string(agent: &mut Agent<NoHooks>, text: &str) -> Value {
        let string = Value::String(agent.new_string_str(text).unwrap());
        held(agent, string)
    }

    fn text(agent: &mut Agent<NoHooks>, value: Value) -> String {
        let Value::String(string) = value else {
            panic!("{value:?} is not a String");
        };
        String::from_utf16(&agent.string_units_for_tests(string).unwrap()).unwrap()
    }

    /// A `valueOf` answering 7.
    fn seven(_cx: &mut Cx<'_, NoHooks>, _call: HostCall<'_>) -> Completion<Value> {
        Ok(Value::Number(7.0))
    }

    /// An ordinary object whose `valueOf` answers 7.
    fn object_of_seven(agent: &mut Agent<NoHooks>, realm: &crate::value::Realm) -> ObjectRef {
        let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let value_of = agent
            .create_host_function(handle, "valueOf", 0, seven, false, None)
            .unwrap();
        let prototype = agent.intrinsic(handle, Intrinsic::ObjectPrototype).unwrap();
        let object = agent
            .ordinary_object_create(Some(prototype), OrdinaryData::Plain)
            .unwrap();
        let key = agent.name_key("valueOf").unwrap();
        agent
            .create_data_property(object, key, Value::Object(value_of))
            .unwrap();
        held(agent, Value::Object(object));
        object
    }

    /// IsLooselyEqual (`#sec-islooselyequal`): each step of the clause, and its symmetry.
    #[test]
    fn loose_equality_table() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let seven = Value::Object(object_of_seven(&mut agent, &realm));
        let other = Value::Object(object_of_seven(&mut agent, &realm));
        let one = string(&mut agent, "1");
        let one_again = string(&mut agent, "1");
        let empty = string(&mut agent, "");
        let blank = string(&mut agent, " \t\n\u{A0}\u{3000}");
        let zero = string(&mut agent, "0");
        let seven_text = string(&mut agent, "7");
        let n = Value::Number;
        let b = Value::Boolean;
        let cases = [
            (Value::Undefined, Value::Null, true),
            (Value::Undefined, Value::Undefined, true),
            (Value::Null, n(0.0), false),
            (Value::Undefined, b(false), false),
            (Value::Null, b(false), false),
            (n(1.0), one, true),
            (one, one_again, true),
            (n(f64::NAN), n(f64::NAN), false),
            (n(0.0), n(-0.0), true),
            (empty, n(0.0), true),
            (blank, n(0.0), true),
            (b(true), n(1.0), true),
            (b(false), zero, true),
            (b(true), one, true),
            (b(true), n(2.0), false),
            (seven, n(7.0), true),
            (seven, seven_text, true),
            (seven, b(true), false),
            (seven, seven, true),
            (seven, other, false),
            (seven, Value::Null, false),
            (one, Value::Null, false),
        ];
        for (x, y, expected) in cases {
            assert_eq!(agent.is_loosely_equal(x, y), Ok(expected), "{x:?} == {y:?}");
            assert_eq!(agent.is_loosely_equal(y, x), Ok(expected), "{y:?} == {x:?}");
        }
    }

    /// ApplyStringOrNumericBinaryOperator (`#sec-applystringornumericbinaryoperator`) and
    /// IsLessThan (`#sec-islessthan`): `+` concatenates when either primitive is a String and
    /// adds otherwise, after ToPrimitive; the other operators are the Number operations;
    /// relational comparison compares Strings by code units and other values as Numbers, NaN
    /// answering undefined.
    #[test]
    fn addition_and_relational() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let seven = Value::Object(object_of_seven(&mut agent, &realm));
        let a = string(&mut agent, "a");
        let two = string(&mut agent, "2");
        let n = Value::Number;
        let add = BinaryOperator::Add;

        assert_eq!(
            agent.apply_string_or_numeric_binary_operator(n(1.0), add, n(2.0)),
            Ok(n(3.0))
        );
        let sum = agent
            .apply_string_or_numeric_binary_operator(a, add, n(1.0))
            .unwrap();
        assert_eq!(text(&mut agent, sum), "a1");
        let sum = agent
            .apply_string_or_numeric_binary_operator(n(1.0), add, two)
            .unwrap();
        assert_eq!(text(&mut agent, sum), "12");
        let sum = agent
            .apply_string_or_numeric_binary_operator(seven, add, a)
            .unwrap();
        assert_eq!(text(&mut agent, sum), "7a");
        assert_eq!(
            agent.apply_string_or_numeric_binary_operator(seven, add, n(1.0)),
            Ok(n(8.0))
        );
        assert_eq!(
            agent.apply_string_or_numeric_binary_operator(Value::Boolean(true), add, Value::Null),
            Ok(n(1.0))
        );
        let undefined_sum = agent
            .apply_string_or_numeric_binary_operator(Value::Undefined, add, n(1.0))
            .unwrap();
        assert!(matches!(undefined_sum, Value::Number(x) if x.is_nan()));
        assert_eq!(
            agent.apply_string_or_numeric_binary_operator(two, BinaryOperator::Multiply, seven),
            Ok(n(14.0))
        );

        for (operator, x, y, expected) in [
            (BinaryOperator::Subtract, 1.0, 3.0, -2.0),
            (BinaryOperator::Divide, 1.0, 0.0, f64::INFINITY),
            (BinaryOperator::Divide, -1.0, 0.0, f64::NEG_INFINITY),
            (BinaryOperator::Remainder, 7.0, 3.0, 1.0),
            (BinaryOperator::Remainder, -7.0, 3.0, -1.0),
            (BinaryOperator::Remainder, 5.5, f64::INFINITY, 5.5),
            (BinaryOperator::LeftShift, 1.0, 31.0, -2_147_483_648.0),
            (BinaryOperator::LeftShift, 1.0, 33.0, 2.0),
            (BinaryOperator::SignedRightShift, -8.0, 1.0, -4.0),
            (
                BinaryOperator::UnsignedRightShift,
                -1.0,
                0.0,
                4_294_967_295.0,
            ),
            (BinaryOperator::BitwiseAnd, 5.0, 3.0, 1.0),
            (BinaryOperator::BitwiseXor, 5.0, 3.0, 6.0),
            (BinaryOperator::BitwiseOr, 5.0, 3.0, 7.0),
            (BinaryOperator::Exponentiate, 2.0, 10.0, 1024.0),
            (BinaryOperator::Exponentiate, f64::NAN, 0.0, 1.0),
            (BinaryOperator::Exponentiate, -0.0, -3.0, f64::NEG_INFINITY),
        ] {
            assert_eq!(
                number_operation(operator, x, y),
                expected,
                "{operator:?} {x} {y}"
            );
        }
        assert!(number_operation(BinaryOperator::Remainder, 1.0, 0.0).is_nan());
        assert!(number_operation(BinaryOperator::Remainder, -0.0, 1.0).is_sign_negative());
        assert!(number_operation(BinaryOperator::Exponentiate, 1.0, f64::INFINITY).is_nan());
        assert!(number_operation(BinaryOperator::Exponentiate, -8.0, 1.0 / 3.0).is_nan());
        assert_eq!(number_unary_minus(0.0).to_bits(), (-0.0_f64).to_bits());
        assert_eq!(number_bitwise_not(5.0), -6.0);

        let ten = string(&mut agent, "10");
        let nine = string(&mut agent, "9");
        let ab = string(&mut agent, "ab");
        assert_eq!(agent.is_less_than(n(1.0), n(2.0), true), Ok(Some(true)));
        assert_eq!(agent.is_less_than(a, ab, true), Ok(Some(true)));
        assert_eq!(agent.is_less_than(ab, a, true), Ok(Some(false)));
        assert_eq!(agent.is_less_than(ten, nine, true), Ok(Some(true)));
        assert_eq!(agent.is_less_than(ten, n(9.0), true), Ok(Some(false)));
        assert_eq!(agent.is_less_than(seven, n(8.0), false), Ok(Some(true)));
        assert_eq!(agent.is_less_than(a, n(1.0), true), Ok(None));
        assert_eq!(agent.is_less_than(n(f64::NAN), n(1.0), true), Ok(None));

        assert_eq!(agent.type_of(Value::Null), Ok("object"));
        assert_eq!(agent.type_of(seven), Ok("object"));
        let Value::Object(seven_object) = seven else {
            unreachable!()
        };
        let key = agent.name_key("valueOf").unwrap();
        let value_of = agent.get(seven_object, key).unwrap();
        assert_eq!(agent.type_of(value_of), Ok("function"));
        assert_eq!(agent.type_of(a), Ok("string"));
        assert_eq!(agent.type_of(Value::Undefined), Ok("undefined"));
    }

    /// Decision 2, `string_length`: a concatenation whose result would be longer than the
    /// bound throws a `RangeError` before the String is allocated; one exactly at the bound is
    /// made.
    #[test]
    fn concatenation_past_string_length_is_range_error() {
        let mut limits = limits();
        limits.string_length = 4;
        let (mut agent, realm) = agent_with_realm_and(limits);
        enter(&mut agent, &realm);
        let realm = agent.heap.roots.get_realm(&realm.0).unwrap();
        let ab = string(&mut agent, "ab");
        let cd = string(&mut agent, "cd");
        let cde = string(&mut agent, "cde");
        let sum = agent
            .apply_string_or_numeric_binary_operator(ab, BinaryOperator::Add, cd)
            .unwrap();
        assert_eq!(text(&mut agent, sum), "abcd");

        let strings_before = agent.heap.strings.slot_count();
        let thrown = agent.apply_string_or_numeric_binary_operator(ab, BinaryOperator::Add, cde);
        let Err(Abrupt::Throw(Value::Object(error))) = thrown else {
            panic!("{thrown:?}");
        };
        let range_error = agent
            .intrinsic(realm, Intrinsic::ErrorPrototype(ErrorKind::RangeError))
            .unwrap();
        assert_eq!(agent.get_prototype_of(error), Ok(Some(range_error)));
        // The only String made is the error's message: the String of 5 code units is not.
        assert_eq!(agent.heap.strings.slot_count(), strings_before + 1);
        let number_sum = agent.apply_string_or_numeric_binary_operator(
            Value::Number(12.0),
            BinaryOperator::Add,
            cde,
        );
        assert!(matches!(number_sum, Err(Abrupt::Throw(_))));
    }
}
