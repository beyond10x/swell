//! String objects (ECMA-262 22.1, `#sec-string-objects`): the `String` constructor
//! (`#sec-string-constructor-string-value`), `String.prototype.constructor`
//! (`#sec-string.prototype.constructor`) and `String.prototype.indexOf`
//! (`#sec-string.prototype.indexof`) with StringIndexOf (`#sec-stringindexof`), and the
//! builder of the Strings the builtins make at run time from other values. Written by
//! `story:harness-builtins`. Slice 2 takes `String.prototype.toString`
//! (`#sec-string.prototype.tostring`) and `String.prototype.valueOf`
//! (`#sec-string.prototype.valueof`) with ThisStringValue (`#sec-thisstringvalue`) early
//! (`docs/design/engine.md`, section 12, "A part before its slice";
//! `story:number-boolean-and-global-functions-early`): with them ToPrimitive of a String object
//! finds its String. The other methods are slice 3, the ones that need Unicode tables slice 5.
//!
//! A String built at run time from other values is bounded by `string_length`
//! (`docs/design/host-interface.md`, section 2): [`StringBuilder`] throws a `RangeError` before
//! its units would pass it, asks the memory budget before it grows, and charges a step per code
//! unit it copies, so the copy is part of the step budget.

use crate::builtins::object::argument;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host, Termination};
use crate::object::ObjectKind;
use crate::object::builtin_function::BuiltinId;
use crate::realm::intrinsics::{Intrinsic, Making};
use crate::value::{ObjectRef, StringRef, SymbolRef, Value};

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StringBuiltin {
    /// `String ( value )` (22.1.1.1).
    Constructor,
    /// `String.prototype.indexOf ( searchString [ , position ] )` (22.1.3.9).
    IndexOf,
    /// `String.prototype.toString ( )` (22.1.3.29).
    ToString,
    /// `String.prototype.valueOf ( )` (22.1.3.35).
    ValueOf,
}

/// The code units of a String being built.
#[derive(Default)]
pub(crate) struct StringBuilder {
    units: Vec<u16>,
}

impl StringBuilder {
    /// An empty builder.
    pub(crate) fn new() -> StringBuilder {
        StringBuilder::default()
    }
}

/// The code units of `text`.
pub(crate) fn units_of(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

impl<H: Host> Agent<H> {
    /// Append `units` to `builder`: a result longer than `string_length` throws a `RangeError`
    /// before anything is copied; one whose String would not fit the memory budget ends the run
    /// in `Terminated(MemoryBudget)` before the buffer grows; a step per code unit.
    pub(crate) fn append_units(
        &mut self,
        builder: &mut StringBuilder,
        units: &[u16],
    ) -> Completion<()> {
        let length = builder.units.len().saturating_add(units.len());
        if u64::try_from(length).unwrap_or(u64::MAX) > self.limits.string_length {
            return Err(self.throw_error(
                ErrorKind::RangeError,
                "the string would be longer than string_length",
            ));
        }
        self.charge_steps(u64::try_from(units.len()).unwrap_or(u64::MAX))?;
        let bytes = u64::try_from(length).unwrap_or(u64::MAX).saturating_mul(2);
        if self.heap.accounting.check(bytes).is_err()
            || builder.units.try_reserve(units.len()).is_err()
        {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        builder.units.extend_from_slice(units);
        Ok(())
    }

    /// Append the code units of `text`, as [`Agent::append_units`].
    pub(crate) fn append_str(&mut self, builder: &mut StringBuilder, text: &str) -> Completion<()> {
        self.append_units(builder, &units_of(text))
    }

    /// Append the code units of `string`, as [`Agent::append_units`].
    pub(crate) fn append_string(
        &mut self,
        builder: &mut StringBuilder,
        string: StringRef,
    ) -> Completion<()> {
        let units = self.string_units(string)?;
        self.append_units(builder, &units)
    }

    /// The String of the units built.
    pub(crate) fn finish_string(&mut self, builder: StringBuilder) -> Completion<StringRef> {
        self.new_string_value(&builder.units)
    }

    /// `%String%` (22.1.1) with `String.prototype.constructor`, `indexOf`, `toString` and
    /// `valueOf`.
    pub(crate) fn create_string_constructor(
        &mut self,
        making: Making,
        string_prototype: ObjectRef,
    ) -> Completion<ObjectRef> {
        let constructor = self.create_builtin_constructor(
            making,
            "String",
            1,
            BuiltinId::String(StringBuiltin::Constructor),
            string_prototype,
        )?;
        for (name, length, id) in [
            ("indexOf", 1, StringBuiltin::IndexOf),
            ("toString", 0, StringBuiltin::ToString),
            ("valueOf", 0, StringBuiltin::ValueOf),
        ] {
            self.define_builtin_method(
                making,
                string_prototype,
                name,
                length,
                BuiltinId::String(id),
            )?;
        }
        Ok(constructor)
    }

    /// The steps of the builtins of this module.
    pub(crate) fn run_string_builtin(
        &mut self,
        id: StringBuiltin,
        _function: ObjectRef,
        this: Value,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        match id {
            StringBuiltin::Constructor => self.string_constructor(args, new_target),
            StringBuiltin::IndexOf => self.string_index_of_method(this, args),
            // `#sec-string.prototype.tostring`: 1. Return ? ThisStringValue(this value).
            StringBuiltin::ToString => self
                .this_string_value(this, "String.prototype.toString")
                .map(Value::String),
            // `#sec-string.prototype.valueof`: 1. Return ? ThisStringValue(this value).
            StringBuiltin::ValueOf => self
                .this_string_value(this, "String.prototype.valueOf")
                .map(Value::String),
        }
    }

    /// ThisStringValue ( arg ) (`#sec-thisstringvalue`), for the method `method`.
    fn this_string_value(&mut self, value: Value, method: &str) -> Completion<StringRef> {
        match value {
            // 1. If arg is a String, return arg.
            Value::String(string) => return Ok(string),
            // 2. If arg is an Object and arg has a [[StringData]] internal slot, then return
            // arg.[[StringData]].
            Value::Object(object) => {
                if let ObjectKind::String(string) = &self.engine_object(object)?.kind {
                    return Ok(string.data);
                }
            }
            _ => {}
        }
        // 3. Throw a TypeError exception.
        Err(self.throw_error(
            ErrorKind::TypeError,
            &format!("{method} requires that 'this' be a String"),
        ))
    }

    /// `String ( value )` (`#sec-string-constructor-string-value`).
    fn string_constructor(
        &mut self,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        // 1. If value is not present, let s be the empty String.
        let string = match args.first() {
            None => self.new_string_str("")?,
            // 2.a. If NewTarget is undefined and value is a Symbol, return
            // SymbolDescriptiveString(value).
            Some(&Value::Symbol(symbol)) if new_target.is_none() => {
                return Ok(Value::String(self.symbol_descriptive_string(symbol)?));
            }
            // 2.b. Let s be ? ToString(value).
            Some(&value) => self.to_string(value)?,
        };
        // 3. If NewTarget is undefined, return s.
        let Some(new_target) = new_target else {
            return Ok(Value::String(string));
        };
        // 4. Return StringCreate(s, ? GetPrototypeFromConstructor(NewTarget,
        // "%String.prototype%")). The Getter of "prototype" may run code, so s is kept.
        let keep = self.keep(Value::String(string));
        let prototype = self.get_prototype_from_constructor(new_target, Intrinsic::StringPrototype);
        self.release(keep);
        let made = self.string_create(string, prototype?)?;
        Ok(Value::Object(made))
    }

    /// SymbolDescriptiveString (`#sec-symboldescriptivestring`): "Symbol(", the description or
    /// the empty String, and ")".
    pub(crate) fn symbol_descriptive_string(&mut self, symbol: SymbolRef) -> Completion<StringRef> {
        let description = match self.heap.symbols.get(symbol.handle) {
            Ok(record) => record.description,
            Err(_) => return Err(self.fault()),
        };
        let mut builder = StringBuilder::new();
        self.append_str(&mut builder, "Symbol(")?;
        if let Some(description) = description {
            self.append_string(&mut builder, description)?;
        }
        self.append_str(&mut builder, ")")?;
        self.finish_string(builder)
    }

    /// `String.prototype.indexOf ( searchString [ , position ] )`
    /// (`#sec-string.prototype.indexof`).
    fn string_index_of_method(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        // 2. Perform ? RequireObjectCoercible(O) (`#sec-requireobjectcoercible`).
        if matches!(this, Value::Undefined | Value::Null) {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "String.prototype.indexOf called on undefined or null",
            ));
        }
        // 3. Let S be ? ToString(O).
        let string = self.to_string(this)?;
        let keep = self.keep(Value::String(string));
        let found = self.string_index_of_steps(string, args);
        self.release(keep);
        found
    }

    /// Steps 4 to 10 of `indexOf`, with S kept.
    fn string_index_of_steps(&mut self, string: StringRef, args: &[Value]) -> Completion<Value> {
        // 4. Let searchStr be ? ToString(searchString).
        let search = self.to_string(argument(args, 0))?;
        let keep = self.keep(Value::String(search));
        // 5. Let pos be ? ToIntegerOrInfinity(position).
        let position = self.to_integer_or_infinity(argument(args, 1));
        self.release(keep);
        let position = position?;
        let units = self.string_units(string)?;
        let search = self.string_units(search)?;
        // 7. Let len be the length of S. 8. Let start be the result of clamping pos between 0
        // and len.
        let length = units.len();
        let start = if position <= 0.0 {
            0
        } else if position >= length as f64 {
            length
        } else {
            // 0 < position < length: an integer that fits.
            position as usize
        };
        Ok(Value::Number(
            match self.string_index_of(&units, &search, start)? {
                Some(index) => index as f64,
                None => -1.0,
            },
        ))
    }

    /// StringIndexOf (`#sec-stringindexof`): the first index at or after `from` where `search`
    /// occurs in `string`. A step per candidate index and per code unit compared.
    pub(crate) fn string_index_of(
        &mut self,
        string: &[u16],
        search: &[u16],
        from: usize,
    ) -> Completion<Option<usize>> {
        let length = string.len();
        // 2. If searchValue is the empty String and fromIndex ≤ len, return fromIndex.
        if search.is_empty() && from <= length {
            return Ok(Some(from));
        }
        // 4. For each integer i such that fromIndex ≤ i ≤ len - searchLen, in ascending order.
        let Some(last) = length.checked_sub(search.len()) else {
            return Ok(None);
        };
        let per_candidate = u64::try_from(search.len()).unwrap_or(u64::MAX).max(1);
        for index in from..=last {
            self.charge_steps(per_candidate)?;
            if string.get(index..index + search.len()) == Some(search) {
                return Ok(Some(index));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::StringBuilder;
    use crate::host::Termination;
    use crate::host::hooks::test_host::{agent_with_realm, agent_with_realm_and, enter, limits};
    use crate::host::outcome::Abrupt;
    use crate::realm::intrinsics::Intrinsic;
    use crate::value::Value;

    /// `#sec-string-constructor-string-value`, step 2.a, with SymbolDescriptiveString
    /// (`#sec-symboldescriptivestring`): `String(symbol)` answers "Symbol(" and its
    /// description and ")"; `new String(symbol)` goes through ToString and throws a TypeError.
    /// A well-known symbol stands for any Symbol: the host interface hands out none.
    #[test]
    fn string_of_a_symbol_is_its_descriptive_string() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let string = agent.intrinsic(handle, Intrinsic::String).unwrap();
        let symbols = agent.well_known().unwrap();
        let symbol = Value::Symbol(symbols.has_instance);
        let Ok(Value::String(text)) =
            agent.call_value(Value::Object(string), Value::Undefined, &[symbol])
        else {
            panic!("String(symbol) is not a String");
        };
        let text = agent.string_units(text).unwrap();
        assert_eq!(
            String::from_utf16_lossy(&text),
            "Symbol(Symbol.hasInstance)"
        );
        let anonymous = agent.heap.new_symbol(None).unwrap();
        let Ok(Value::String(text)) = agent.call_value(
            Value::Object(string),
            Value::Undefined,
            &[Value::Symbol(anonymous)],
        ) else {
            panic!("String(symbol) is not a String");
        };
        let text = agent.string_units(text).unwrap();
        assert_eq!(String::from_utf16_lossy(&text), "Symbol()");
        let thrown = agent.construct_value(string, &[symbol], None);
        assert!(matches!(thrown, Err(Abrupt::Throw(_))), "{thrown:?}");
    }

    /// The builder of the Strings `join`, `JSON.stringify`, `bind` and `Object.prototype.toString`
    /// make asks the memory budget before it grows (`docs/design/engine.md`, decision 4, "Memory
    /// budget"): with `string_length` far above it, code units whose String would not fit
    /// `memory_bytes` end the run in `Terminated(MemoryBudget)` before the buffer grows, and
    /// nothing is charged; within the budget they are appended.
    #[test]
    fn builder_past_the_memory_budget_terminates() {
        let mut limits = limits();
        limits.memory_bytes = 1 << 22;
        limits.string_length = 1 << 30;
        limits.steps_per_entry = 1 << 30;
        let (mut agent, realm) = agent_with_realm_and(limits);
        enter(&mut agent, &realm);
        let used = agent.heap.accounting.used();
        let mut builder = StringBuilder::new();
        agent.append_units(&mut builder, &[0x61; 1000]).unwrap();
        let over = vec![0x61; 1 << 21];
        let ended = agent.append_units(&mut builder, &over);
        assert!(
            matches!(ended, Err(Abrupt::Terminated(Termination::MemoryBudget))),
            "{ended:?}"
        );
        assert_eq!(builder.units.len(), 1000);
        assert_eq!(agent.heap.accounting.used(), used);
    }
}
