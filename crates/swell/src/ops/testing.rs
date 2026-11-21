//! Testing and comparison operations (ECMA-262 7.2, `#sec-testing-and-comparison-operations`):
//! IsCallable, IsConstructor, IsArray, SameValue, SameValueZero, IsLooselyEqual,
//! IsStrictlyEqual and IsLessThan. No Proxy exists before slice 9 and no BigInt before slice 12,
//! so the steps for them are not reached here. Written by `story:object-model-and-realms`.

use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{Host, PreferredType};
use crate::object::KindTag;
use crate::value::{StringRef, Value};

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
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "Array.isArray: story:harness-builtins")
    )]
    pub(crate) fn is_array(&mut self, value: Value) -> Completion<bool> {
        match value {
            Value::Object(object) => Ok(self.kind_of(object)? == KindTag::Array),
            _ => Ok(false),
        }
    }

    /// Whether two Strings have the same code units.
    pub(crate) fn string_equals(&mut self, x: StringRef, y: StringRef) -> Completion<bool> {
        if x == y {
            return Ok(true);
        }
        Ok(self.string_units(x)? == self.string_units(y)?)
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
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "Array.prototype.includes: story:harness-builtins")
    )]
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
            let same_type = std::mem::discriminant(&x) == std::mem::discriminant(&y);
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
            // 3. Both Strings: the code units compared in order; a prefix is less.
            let (px, py) = (self.string_units(px)?, self.string_units(py)?);
            let compared = u64::try_from(px.len().min(py.len())).unwrap_or(u64::MAX);
            self.charge_steps(compared)?;
            return Ok(Some(px < py));
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
    use crate::value::Value;

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
}
