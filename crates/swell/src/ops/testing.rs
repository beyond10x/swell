//! Testing and comparison operations (ECMA-262 7.2, `#sec-testing-and-comparison-operations`):
//! IsCallable, IsConstructor, IsArray, SameValue, SameValueZero, IsLooselyEqual,
//! IsStrictlyEqual and IsLessThan. No Proxy exists before slice 9 and no BigInt before slice 12,
//! so the steps for them are not reached here. Written by `story:object-model-and-realms`.
//!
//! Two Strings are compared in place, with no copy. Equality charges a step per code unit past
//! [`FREE_CODE_UNITS`], IsLessThan a step per code unit of the shorter String, each before a
//! unit is read (`docs/design/engine.md`, "The step budget").

use core::cmp::Ordering;

use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{Host, PreferredType};
use crate::object::KindTag;
use crate::value::{StringRef, Value};

/// The code units of a String that a comparison of two Strings, or the String made a property
/// key, reads without a step of its own; past them, a step per code unit
/// (`docs/design/engine.md`, "The step budget").
pub(crate) const FREE_CODE_UNITS: usize = 32;

/// Number::sameValue (`#sec-numeric-types-number-sameValue`).
pub(crate) fn number_same_value(x: f64, y: f64) -> bool {
    (x.is_nan() && y.is_nan()) || (x == y && x.is_sign_negative() == y.is_sign_negative())
}

impl<H: Host> Agent<H> {
    /// IsCallable (`#sec-iscallable`).
    pub(crate) fn is_callable(&mut self, value: Value) -> Completion<bool> {
        match value {
            Value::Object(object) => self.has_call(object),
            _ => Ok(false),
        }
    }

    /// IsConstructor (`#sec-isconstructor`).
    pub(crate) fn is_constructor(&mut self, value: Value) -> Completion<bool> {
        match value {
            Value::Object(object) => self.has_construct(object),
            _ => Ok(false),
        }
    }

    /// IsArray (`#sec-isarray`).
    pub(crate) fn is_array(&mut self, value: Value) -> Completion<bool> {
        match value {
            Value::Object(object) => Ok(self.kind_of(object)? == KindTag::Array),
            _ => Ok(false),
        }
    }

    /// A step per code unit of `length` past [`FREE_CODE_UNITS`], before the units are read; none
    /// for `length` of at most that.
    pub(crate) fn charge_code_units(&mut self, length: usize) -> Completion<()> {
        match length.checked_sub(FREE_CODE_UNITS) {
            Some(past) if past > 0 => self.charge_steps(u64::try_from(past).unwrap_or(u64::MAX)),
            _ => Ok(()),
        }
    }

    /// Whether two Strings have the same code units (SameValueNonNumber,
    /// `#sec-samevaluenonnumber`, step 5). One String, or two of different lengths, answer
    /// without a step; otherwise a step per code unit past [`FREE_CODE_UNITS`] is charged before
    /// a unit is read, and the two records are compared in place.
    pub(crate) fn string_equals(&mut self, x: StringRef, y: StringRef) -> Completion<bool> {
        if x == y {
            return Ok(true);
        }
        let length = self.string_length(x)?;
        if self.string_length(y)? != length {
            return Ok(false);
        }
        self.charge_code_units(length)?;
        let equal = match (
            self.heap.strings.get(x.handle),
            self.heap.strings.get(y.handle),
        ) {
            (Ok(x), Ok(y)) => Some(x.equals_string(y)),
            _ => None,
        };
        equal.ok_or_else(|| self.fault())
    }

    /// SameValueNonNumber (`#sec-samevaluenonnumber`), for values of one type.
    fn same_value_non_number(&mut self, x: Value, y: Value) -> Completion<bool> {
        Ok(match (x, y) {
            (Value::Undefined, Value::Undefined) | (Value::Null, Value::Null) => true,
            (Value::Boolean(x), Value::Boolean(y)) => x == y,
            (Value::String(x), Value::String(y)) => self.string_equals(x, y)?,
            (Value::Symbol(x), Value::Symbol(y)) => x == y,
            (Value::Object(x), Value::Object(y)) => x == y,
            (Value::BigInt(_), Value::BigInt(_)) => return Err(self.fault()),
            _ => false,
        })
    }

    /// SameValue (`#sec-samevalue`).
    pub(crate) fn same_value(&mut self, x: Value, y: Value) -> Completion<bool> {
        match (x, y) {
            (Value::Number(x), Value::Number(y)) => Ok(number_same_value(x, y)),
            _ => self.same_value_non_number(x, y),
        }
    }

    /// SameValueZero (`#sec-samevaluezero`).
    pub(crate) fn same_value_zero(&mut self, x: Value, y: Value) -> Completion<bool> {
        match (x, y) {
            (Value::Number(x), Value::Number(y)) => Ok((x.is_nan() && y.is_nan()) || x == y),
            _ => self.same_value_non_number(x, y),
        }
    }

    /// IsStrictlyEqual (`#sec-isstrictlyequal`).
    pub(crate) fn is_strictly_equal(&mut self, x: Value, y: Value) -> Completion<bool> {
        match (x, y) {
            // Number::equal: NaN is not equal to itself, +0 equals -0.
            (Value::Number(x), Value::Number(y)) => Ok(x == y),
            _ => self.same_value_non_number(x, y),
        }
    }

    /// IsLooselyEqual (`#sec-islooselyequal`), as a loop: each turn converts one side, so it
    /// ends within three turns.
    pub(crate) fn is_loosely_equal(&mut self, x: Value, y: Value) -> Completion<bool> {
        let (mut x, mut y) = (x, y);
        loop {
            let same_type = core::mem::discriminant(&x) == core::mem::discriminant(&y);
            if same_type {
                return self.is_strictly_equal(x, y);
            }
            match (x, y) {
                (Value::Undefined | Value::Null, Value::Undefined | Value::Null) => {
                    return Ok(true);
                }
                (Value::Number(_), Value::String(string)) => {
                    y = Value::Number(self.string_to_number(string)?);
                }
                (Value::String(string), Value::Number(_)) => {
                    x = Value::Number(self.string_to_number(string)?);
                }
                (Value::Boolean(_), _) => x = Value::Number(self.to_number(x)?),
                (_, Value::Boolean(_)) => y = Value::Number(self.to_number(y)?),
                (
                    Value::String(_) | Value::Number(_) | Value::BigInt(_) | Value::Symbol(_),
                    Value::Object(_),
                ) => {
                    let keep = self.keep(x);
                    let primitive = self.to_primitive(y, None);
                    self.release(keep);
                    y = primitive?;
                }
                (
                    Value::Object(_),
                    Value::String(_) | Value::Number(_) | Value::BigInt(_) | Value::Symbol(_),
                ) => {
                    let keep = self.keep(y);
                    let primitive = self.to_primitive(x, None);
                    self.release(keep);
                    x = primitive?;
                }
                _ => return Ok(false),
            }
        }
    }

    /// IsLessThan (`#sec-islessthan`): `None` for undefined (a NaN operand).
    pub(crate) fn is_less_than(
        &mut self,
        x: Value,
        y: Value,
        left_first: bool,
    ) -> Completion<Option<bool>> {
        let (px, py) = if left_first {
            let px = self.to_primitive(x, Some(PreferredType::Number))?;
            let keep = self.keep(px);
            let py = self.to_primitive(y, Some(PreferredType::Number));
            self.release(keep);
            (px, py?)
        } else {
            let py = self.to_primitive(y, Some(PreferredType::Number))?;
            let keep = self.keep(py);
            let px = self.to_primitive(x, Some(PreferredType::Number));
            self.release(keep);
            (px?, py)
        };
        if let (Value::String(px), Value::String(py)) = (px, py) {
            // 3. Both Strings: the code units compared in order; a prefix is less. The shorter
            // length is charged before a unit is read, and the records are compared in place.
            let compared = self.string_length(px)?.min(self.string_length(py)?);
            self.charge_steps(u64::try_from(compared).unwrap_or(u64::MAX))?;
            let less = match (
                self.heap.strings.get(px.handle),
                self.heap.strings.get(py.handle),
            ) {
                (Ok(x), Ok(y)) => Some(x.cmp_string(y) == Ordering::Less),
                _ => None,
            };
            return less.map(Some).ok_or_else(|| self.fault());
        }
        let nx = self.to_number(px)?;
        let ny = self.to_number(py)?;
        if nx.is_nan() || ny.is_nan() {
            return Ok(None);
        }
        Ok(Some(nx < ny))
    }
}

#[cfg(test)]
mod tests {
    use crate::host::hooks::test_host::{agent_with_realm, enter};
    use crate::host::outcome::{Abrupt, Termination};
    use crate::object::string::copies;
    use crate::value::Value;
    use alloc::vec;

    /// SameValue, SameValueZero and IsStrictlyEqual differ on NaN and the signed zeros
    /// (`#sec-samevalue`, `#sec-samevaluezero`, `#sec-isstrictlyequal`); two Strings of the
    /// same code units are the same value; IsArray is true for an Array only.
    #[test]
    fn same_value_family_and_is_array() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let (nan, zero, minus) = (
            Value::Number(f64::NAN),
            Value::Number(0.0),
            Value::Number(-0.0),
        );
        assert_eq!(agent.same_value(nan, nan), Ok(true));
        assert_eq!(agent.same_value(zero, minus), Ok(false));
        assert_eq!(agent.same_value_zero(nan, nan), Ok(true));
        assert_eq!(agent.same_value_zero(zero, minus), Ok(true));
        assert_eq!(agent.is_strictly_equal(nan, nan), Ok(false));
        assert_eq!(agent.is_strictly_equal(zero, minus), Ok(true));
        let a = Value::String(agent.new_string_str("a").unwrap());
        let b = Value::String(agent.new_string_str("a").unwrap());
        assert_eq!(agent.same_value(a, b), Ok(true));
        assert_eq!(agent.is_strictly_equal(a, Value::Number(1.0)), Ok(false));
        let array = agent.array_create(0.0, None).unwrap();
        assert_eq!(agent.is_array(Value::Object(array)), Ok(true));
        let wrapper = agent.to_object(Value::Boolean(true)).unwrap();
        assert_eq!(agent.is_array(Value::Object(wrapper)), Ok(false));
        assert_eq!(agent.is_array(a), Ok(false));
        assert_eq!(agent.is_constructor(Value::Object(array)), Ok(false));
        assert_eq!(agent.is_constructor(a), Ok(false));
    }

    /// The code units of the long Strings of the tests below.
    const LONG: usize = 1_000_000;

    /// IsLessThan (`#sec-islessthan`, step 3) compares two Strings in place: `"a" < s` with `s`
    /// of 1,000,000 code units answers true, and no copy of a String's units was made.
    #[test]
    fn is_less_than_copies_nothing() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let a = Value::String(agent.new_string_str("a").unwrap());
        let s = Value::String(agent.new_string_value(&vec![0x61; LONG]).unwrap());
        agent.keep(a);
        agent.keep(s);
        copies::reset();
        assert_eq!(agent.is_less_than(a, s, true), Ok(Some(true)));
        assert_eq!(copies::longest(), 0);
    }

    /// StringToNumber (`#sec-stringtonumber`) charges a step per code unit before it copies
    /// them: `+s` of 1,000,000 code units with 1,000 steps left ends in
    /// `Terminated(StepBudget)`, and no copy of its units was made.
    #[test]
    fn string_to_number_is_charged_before_its_copy() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let s = Value::String(agent.new_string_value(&vec![0x31; LONG]).unwrap());
        agent.keep(s);
        copies::reset();
        agent.limits.steps_per_entry = agent.steps + 1_000;
        assert_eq!(
            agent.to_number(s),
            Err(Abrupt::Terminated(Termination::StepBudget))
        );
        assert_eq!(copies::longest(), 0);
    }
}
