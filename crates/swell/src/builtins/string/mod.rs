//! String objects (ECMA-262 22.1, `#sec-string-objects`): the `String` constructor
//! (`#sec-string-constructor-string-value`), `String.prototype.constructor`
//! (`#sec-string.prototype.constructor`) and `String.prototype.indexOf`
//! (`#sec-string.prototype.indexof`) with StringIndexOf (`#sec-stringindexof`), and the
//! builder of the Strings the builtins make at run time from other values. Written by
//! `story:harness-builtins`. `String.prototype[@@iterator]`
//! (`#sec-string.prototype-%symbol.iterator%`) is
//! `story:iterator-protocol-and-array-string-iterators`, its steps in `builtins/iterator.rs`.
//! Slice 2 takes `String.prototype.toString`
//! (`#sec-string.prototype.tostring`) and `String.prototype.valueOf`
//! (`#sec-string.prototype.valueof`) with ThisStringValue (`#sec-thisstringvalue`) early
//! (`docs/design/engine.md`, section 12, "A part before its slice";
//! `story:number-boolean-and-global-functions-early`): with them ToPrimitive of a String object
//! finds its String. The other methods are slice 3, the ones that need Unicode tables slice 5;
//! those of slice 3 are in the group modules beside this file (`story:builtin-group-modules`):
//! `access.rs` (`story:string-access-and-search-methods`) and `transform.rs`
//! (`story:string-transform-and-compare-methods`); those of slice 5 in `replace.rs`
//! (`story:string-replace-replace-all-and-split`) and `pattern.rs`, `match` and `search`
//! (`story:regexp-symbol-methods-and-string-match-search`) and `matchAll`
//! (`story:regexp-match-all-and-escape`). Their keys follow the keys of this
//! file on `%String%` and `%String.prototype%`, in that order of the groups. Every method of
//! `String.prototype` but `toString` and `valueOf` starts with [`Agent::this_string`]. This
//! file also has CodePointAt (`#sec-codepointat`) and UTF16EncodeCodePoint
//! (`#sec-utf16encodecodepoint`), functions of code units.
//!
//! A String built at run time from other values is bounded by `string_length`
//! (`docs/design/host-interface.md`, section 2): [`StringBuilder`] throws a `RangeError` before
//! its units would pass it, asks the memory budget before it grows, and charges a step per code
//! unit it copies, so the copy is part of the step budget.

use alloc::format;
use alloc::vec::Vec;

pub(crate) mod access;
pub(crate) mod pattern;
pub(crate) mod replace;
pub(crate) mod transform;

use crate::builtins::object::argument;
use crate::heap::accounting::ChargedVec;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};
use crate::object::ObjectKind;
use crate::object::builtin_function::BuiltinId;
use crate::ops::conversion::clamp_index;
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
    /// `String.prototype [ %Symbol.iterator% ] ( )` (22.1.3.36)
    /// (`story:iterator-protocol-and-array-string-iterators`, `builtins/iterator.rs`).
    Iterator,
    /// A builtin of the group `access.rs`.
    Access(access::AccessBuiltin),
    /// A builtin of the group `transform.rs`.
    Transform(transform::TransformBuiltin),
    /// A builtin of the group `replace.rs`.
    Replace(replace::ReplaceBuiltin),
    /// A builtin of the group `pattern.rs`.
    Pattern(pattern::PatternBuiltin),
}

/// The code units of a String being built, charged by their capacity while the builder lives.
#[derive(Default)]
pub(crate) struct StringBuilder {
    units: ChargedVec<u16>,
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

/// The Record CodePointAt answers (`#sec-codepointat`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CodePoint {
    /// [[CodePoint]].
    pub(crate) code_point: u32,
    /// [[CodeUnitCount]]: 1 or 2.
    pub(crate) code_unit_count: usize,
    /// [[IsUnpairedSurrogate]].
    pub(crate) is_unpaired_surrogate: bool,
}

/// CodePointAt ( string, position ) (`#sec-codepointat`): the code point of `string` that
/// starts at the code unit at `position`. `None` where step 2's assertion (position < size)
/// does not hold. Touches no heap.
pub(crate) fn code_point_at(string: &[u16], position: usize) -> Option<CodePoint> {
    // A leading surrogate is 0xD800 to 0xDBFF, a trailing one 0xDC00 to 0xDFFF (6.1.4).
    let is_leading_surrogate = |unit: u16| (0xD800..=0xDBFF).contains(&unit);
    let is_trailing_surrogate = |unit: u16| (0xDC00..=0xDFFF).contains(&unit);
    // 3. Let first be the code unit at index position within string.
    let first = *string.get(position)?;
    // 4. Let cp be the code point whose numeric value is the numeric value of first.
    let unpaired = CodePoint {
        code_point: u32::from(first),
        code_unit_count: 1,
        is_unpaired_surrogate: true,
    };
    // 5. If first is neither a leading surrogate nor a trailing surrogate, then return the
    // Record { [[CodePoint]]: cp, [[CodeUnitCount]]: 1, [[IsUnpairedSurrogate]]: false }.
    if !is_leading_surrogate(first) && !is_trailing_surrogate(first) {
        return Some(CodePoint {
            is_unpaired_surrogate: false,
            ..unpaired
        });
    }
    // 6. If first is a trailing surrogate or position + 1 = size, then return the Record {
    // [[CodePoint]]: cp, [[CodeUnitCount]]: 1, [[IsUnpairedSurrogate]]: true }.
    // 7. Let second be the code unit at index position + 1 within string.
    let Some(&second) = string
        .get(position + 1)
        .filter(|_| !is_trailing_surrogate(first))
    else {
        return Some(unpaired);
    };
    // 8. If second is not a trailing surrogate, then return the Record { [[CodePoint]]: cp,
    // [[CodeUnitCount]]: 1, [[IsUnpairedSurrogate]]: true }.
    if !is_trailing_surrogate(second) {
        return Some(unpaired);
    }
    // 9. Set cp to UTF16SurrogatePairToCodePoint(first, second)
    // (`#sec-utf16decodesurrogatepair`): (lead - 0xD800) × 0x400 + (trail - 0xDC00) + 0x10000.
    let code_point = (u32::from(first) - 0xD800) * 0x400 + (u32::from(second) - 0xDC00) + 0x10000;
    // 10. Return the Record { [[CodePoint]]: cp, [[CodeUnitCount]]: 2,
    // [[IsUnpairedSurrogate]]: false }.
    Some(CodePoint {
        code_point,
        code_unit_count: 2,
        is_unpaired_surrogate: false,
    })
}

/// The code units of a code point, as UTF16EncodeCodePoint answers them: one or two.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EncodedCodePoint {
    units: [u16; 2],
    count: usize,
}

impl EncodedCodePoint {
    /// The code units, one or two.
    pub(crate) fn units(&self) -> &[u16] {
        self.units.get(..self.count).unwrap_or(&[])
    }
}

/// UTF16EncodeCodePoint ( cp ) (`#sec-utf16encodecodepoint`): the code units of `code_point`.
/// `None` where step 1's assertion (0 ≤ cp ≤ 0x10FFFF) does not hold. Touches no heap.
pub(crate) fn utf16_encode_code_point(code_point: u32) -> Option<EncodedCodePoint> {
    // 1. Assert: 0 ≤ cp ≤ 0x10FFFF.
    if code_point > 0x10FFFF {
        return None;
    }
    // 2. If cp ≤ 0xFFFF, return the String value consisting of the code unit whose numeric
    // value is cp.
    if let Ok(unit) = u16::try_from(code_point) {
        return Some(EncodedCodePoint {
            units: [unit, 0],
            count: 1,
        });
    }
    // 3. Let cu1 be the code unit whose numeric value is floor((cp - 0x10000) / 0x400) +
    // 0xD800. 4. Let cu2 be the code unit whose numeric value is ((cp - 0x10000) modulo
    // 0x400) + 0xDC00. Both are below 0x10000, since cp - 0x10000 < 0x100000.
    let offset = code_point - 0x10000;
    let (Ok(cu1), Ok(cu2)) = (
        u16::try_from(offset / 0x400 + 0xD800),
        u16::try_from(offset % 0x400 + 0xDC00),
    ) else {
        return None;
    };
    // 5. Return the string-concatenation of cu1 and cu2.
    Some(EncodedCodePoint {
        units: [cu1, cu2],
        count: 2,
    })
}

impl<H: Host> Agent<H> {
    /// Append `units` to `builder`: a result longer than `string_length` throws a `RangeError`
    /// before anything is copied; a step per code unit; then the builder's growth is charged to
    /// the memory budget before it is made ([`Agent::reserve_builder`]), and one the budget
    /// refuses ends the run in `Terminated(MemoryBudget)` before the buffer grows.
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
        self.reserve_builder(builder, units.len())?;
        if builder.units.extend_from_slice(units).is_err() {
            return Err(self.fault());
        }
        Ok(())
    }

    /// Room in `builder` for `more` code units ([`Agent::reserve_vec`]), its doubled growth
    /// capped at `string_length` units and charged to the memory budget before it is made.
    pub(crate) fn reserve_builder(
        &mut self,
        builder: &mut StringBuilder,
        more: usize,
    ) -> Completion<()> {
        let most = usize::try_from(self.limits.string_length).unwrap_or(usize::MAX);
        self.reserve_vec_within(&mut builder.units, more, most)
    }

    /// Append the code units of `text`, as [`Agent::append_units`].
    pub(crate) fn append_str(&mut self, builder: &mut StringBuilder, text: &str) -> Completion<()> {
        self.append_units(builder, &units_of(text))
    }

    /// Append the code units of `string`, as [`Agent::append_units`], from its length: the
    /// `RangeError` past `string_length`, the step per code unit and the memory budget come
    /// before a unit is read, and the units are then read from the String in place into the
    /// builder, with no copy between.
    pub(crate) fn append_string(
        &mut self,
        builder: &mut StringBuilder,
        string: StringRef,
    ) -> Completion<()> {
        let count = self.string_length(string)?;
        self.append_string_range(builder, string, 0, count)
    }

    /// Append the code units of `string` from `from` to `to`, as [`Agent::append_string`]. A
    /// range that is not inside the String is a broken invariant.
    pub(crate) fn append_string_range(
        &mut self,
        builder: &mut StringBuilder,
        string: StringRef,
        from: usize,
        to: usize,
    ) -> Completion<()> {
        let count = to.saturating_sub(from);
        let length = builder.units.len().saturating_add(count);
        if u64::try_from(length).unwrap_or(u64::MAX) > self.limits.string_length {
            return Err(self.throw_error(
                ErrorKind::RangeError,
                "the string would be longer than string_length",
            ));
        }
        self.charge_steps(u64::try_from(count).unwrap_or(u64::MAX))?;
        self.reserve_builder(builder, count)?;
        self.extend_units(&mut builder.units, string, from, to)
    }

    /// The String of the units built: the units are taken out and the builder's charge is
    /// given back before the String is made.
    pub(crate) fn finish_string(&mut self, builder: StringBuilder) -> Completion<StringRef> {
        let units = builder.units.into_vec();
        self.new_string_value(&units)
    }

    /// `%String%` (22.1.1) with `String.prototype.constructor`, `indexOf`, `toString` and
    /// `valueOf`; then the methods of each group on both, and the groups' data properties
    /// keyed by a well-known symbol on `%String.prototype%`, the groups in the order `access`,
    /// `transform`, `replace`, `pattern`.
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
        // 22.1.3.36: named "[Symbol.iterator]", { [[Writable]]: true, [[Enumerable]]: false,
        // [[Configurable]]: true } (clause 18).
        let iterator = self.create_builtin(
            making,
            "[Symbol.iterator]",
            0,
            BuiltinId::String(StringBuiltin::Iterator),
        )?;
        self.add_property(
            string_prototype,
            crate::object::property::Key::Symbol(making.symbols.iterator),
            crate::object::property::Property::data(Value::Object(iterator), true, false, true),
        )?;
        for (object, on_prototype) in [(constructor, false), (string_prototype, true)] {
            self.define_group_methods(
                making,
                object,
                if on_prototype {
                    access::PROTOTYPE_METHODS
                } else {
                    access::CONSTRUCTOR_METHODS
                },
                |id| BuiltinId::String(StringBuiltin::Access(id)),
            )?;
            self.define_group_methods(
                making,
                object,
                if on_prototype {
                    transform::PROTOTYPE_METHODS
                } else {
                    transform::CONSTRUCTOR_METHODS
                },
                |id| BuiltinId::String(StringBuiltin::Transform(id)),
            )?;
            self.define_group_methods(
                making,
                object,
                if on_prototype {
                    replace::PROTOTYPE_METHODS
                } else {
                    replace::CONSTRUCTOR_METHODS
                },
                |id| BuiltinId::String(StringBuiltin::Replace(id)),
            )?;
            self.define_group_methods(
                making,
                object,
                if on_prototype {
                    pattern::PROTOTYPE_METHODS
                } else {
                    pattern::CONSTRUCTOR_METHODS
                },
                |id| BuiltinId::String(StringBuiltin::Pattern(id)),
            )?;
        }
        self.define_symbol_data(making, string_prototype, &access::prototype_symbol_data())?;
        self.define_symbol_data(
            making,
            string_prototype,
            &transform::prototype_symbol_data(),
        )?;
        self.define_symbol_data(making, string_prototype, &replace::prototype_symbol_data())?;
        self.define_symbol_data(making, string_prototype, &pattern::prototype_symbol_data())?;
        Ok(constructor)
    }

    /// The steps of the builtins of this module.
    pub(crate) fn run_string_builtin(
        &mut self,
        id: StringBuiltin,
        function: ObjectRef,
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
            StringBuiltin::Iterator => self.string_iterator_of(this),
            StringBuiltin::Access(id) => {
                self.run_string_access_builtin(id, function, this, args, new_target)
            }
            StringBuiltin::Transform(id) => {
                self.run_string_transform_builtin(id, function, this, args, new_target)
            }
            StringBuiltin::Replace(id) => {
                self.run_string_replace_builtin(id, function, this, args, new_target)
            }
            StringBuiltin::Pattern(id) => {
                self.run_string_pattern_builtin(id, function, this, args, new_target)
            }
        }
    }

    /// The first two steps of every method of `String.prototype` but `toString` and `valueOf`
    /// (`#sec-string.prototype.indexof` and the others of 22.1.3), for the method `method`:
    /// "Let O be the this value. Perform ? RequireObjectCoercible(O)"
    /// (`#sec-requireobjectcoercible`), "Let S be ? ToString(O)". ToString may run code; the
    /// caller keeps S across what runs after.
    pub(crate) fn this_string(&mut self, this: Value, method: &str) -> Completion<StringRef> {
        // RequireObjectCoercible: 1. If arg is either undefined or null, throw a TypeError
        // exception.
        if matches!(this, Value::Undefined | Value::Null) {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                &format!("{method} called on undefined or null"),
            ));
        }
        self.to_string(this)
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
        // 1, 2, 3. Let O be the this value; ? RequireObjectCoercible(O); let S be ? ToString(O).
        let string = self.this_string(this, "String.prototype.indexOf")?;
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
        // 7. Let len be the length of S. 8. Let start be the result of clamping pos between 0
        // and len.
        let length = self.string_length(string)?;
        let start = clamp_index(position, length);
        // 9. Let result be StringIndexOf(S, searchStr, start): the search of the code units from
        // start on, from their first, each read charged a step before it is copied
        // (`access.rs`, [`Agent::read_units`]).
        let search_length = self.string_length(search)?;
        let units = self.read_units(string, start, length)?;
        let search = self.read_units(search, 0, search_length)?;
        // 10. If result is not-found, return -1𝔽. 11. Return 𝔽(result). An index of a String is
        // below 2^53, so the conversion is exact.
        Ok(Value::Number(
            match self.string_index_of(&units, &search, 0)? {
                Some(index) => (start + index) as f64,
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
    use super::{
        CodePoint, StringBuilder, StringBuiltin, access, code_point_at, pattern, replace,
        transform, utf16_encode_code_point,
    };
    use crate::builtins::group_testing::{GroupMethod, check_group_keys, group_methods};
    use crate::host::Termination;
    use crate::host::hooks::test_host::{agent_with_realm, agent_with_realm_and, enter, limits};
    use crate::host::outcome::Abrupt;
    use crate::object::builtin_function::BuiltinId;
    use crate::realm::intrinsics::Intrinsic;
    use crate::value::Value;
    use alloc::string::String;
    use alloc::string::ToString;
    use alloc::vec;
    use alloc::vec::Vec;

    /// The order of keys of `String` and `String.prototype` (`story:builtin-group-modules`):
    /// the keys of the base in the base's order, then the keys of the groups `access`,
    /// `transform`, `replace` and `pattern`, and nothing else, each a method of its group that
    /// runs through its function object.
    #[test]
    fn every_group_module_installs_and_runs_through_its_constructor() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let constructor = agent.intrinsic(handle, Intrinsic::String).unwrap();
        let prototype = agent.intrinsic(handle, Intrinsic::StringPrototype).unwrap();
        let constructor_groups: Vec<GroupMethod> = [
            group_methods(access::CONSTRUCTOR_METHODS, |i| {
                BuiltinId::String(StringBuiltin::Access(i))
            }),
            group_methods(transform::CONSTRUCTOR_METHODS, |i| {
                BuiltinId::String(StringBuiltin::Transform(i))
            }),
            group_methods(replace::CONSTRUCTOR_METHODS, |i| {
                BuiltinId::String(StringBuiltin::Replace(i))
            }),
            group_methods(pattern::CONSTRUCTOR_METHODS, |i| {
                BuiltinId::String(StringBuiltin::Pattern(i))
            }),
        ]
        .concat();
        let prototype_groups: Vec<GroupMethod> = [
            group_methods(access::PROTOTYPE_METHODS, |i| {
                BuiltinId::String(StringBuiltin::Access(i))
            }),
            group_methods(transform::PROTOTYPE_METHODS, |i| {
                BuiltinId::String(StringBuiltin::Transform(i))
            }),
            group_methods(replace::PROTOTYPE_METHODS, |i| {
                BuiltinId::String(StringBuiltin::Replace(i))
            }),
            group_methods(pattern::PROTOTYPE_METHODS, |i| {
                BuiltinId::String(StringBuiltin::Pattern(i))
            }),
        ]
        .concat();
        check_group_keys(
            &mut agent,
            constructor,
            &["length", "name", "prototype"],
            &constructor_groups,
            &[],
        );
        check_group_keys(
            &mut agent,
            prototype,
            &["length", "constructor", "indexOf", "toString", "valueOf"],
            &prototype_groups,
            &[],
        );
    }

    /// CodePointAt (`#sec-codepointat`): a code unit that is no surrogate is its own code
    /// point; a leading surrogate followed by a trailing one is the pair's code point
    /// (UTF16SurrogatePairToCodePoint, `#sec-utf16decodesurrogatepair`), two units; a leading
    /// surrogate at the end, a leading surrogate followed by a unit that is not a trailing
    /// surrogate, and a trailing surrogate are unpaired, one unit, the surrogate's own value. A
    /// position at or past the end has no code point.
    #[test]
    fn code_point_at_reads_a_pair_and_a_lone_surrogate_of_each_kind() {
        let point = |code_point, code_unit_count, is_unpaired_surrogate| {
            Some(CodePoint {
                code_point,
                code_unit_count,
                is_unpaired_surrogate,
            })
        };
        // "a", U+1F600 as D83D DE00, a lone trailing DC00, a leading D800 before "b", "b",
        // and a leading DBFF at the end.
        let string = [0x61, 0xD83D, 0xDE00, 0xDC00, 0xD800, 0x62, 0xDBFF];
        assert_eq!(code_point_at(&string, 0), point(0x61, 1, false));
        assert_eq!(code_point_at(&string, 1), point(0x1F600, 2, false));
        // The trailing half of the pair, read on its own, is unpaired.
        assert_eq!(code_point_at(&string, 2), point(0xDE00, 1, true));
        assert_eq!(code_point_at(&string, 3), point(0xDC00, 1, true));
        assert_eq!(code_point_at(&string, 4), point(0xD800, 1, true));
        assert_eq!(code_point_at(&string, 5), point(0x62, 1, false));
        assert_eq!(code_point_at(&string, 6), point(0xDBFF, 1, true));
        assert_eq!(code_point_at(&string, 7), None);
        assert_eq!(code_point_at(&[], 0), None);
        // A leading surrogate before a leading surrogate is unpaired; the edges of the ranges.
        assert_eq!(code_point_at(&[0xD800, 0xD800], 0), point(0xD800, 1, true));
        assert_eq!(code_point_at(&[0xD7FF, 0xDC00], 0), point(0xD7FF, 1, false));
        assert_eq!(code_point_at(&[0xE000], 0), point(0xE000, 1, false));
        assert_eq!(
            code_point_at(&[0xD800, 0xDC00], 0),
            point(0x10000, 2, false)
        );
        assert_eq!(
            code_point_at(&[0xDBFF, 0xDFFF], 0),
            point(0x10FFFF, 2, false)
        );
    }

    /// UTF16EncodeCodePoint (`#sec-utf16encodecodepoint`) and CodePointAt: every Unicode
    /// scalar value (0 to 0x10FFFF without the surrogates) is encoded in one unit up to
    /// 0xFFFF and two above, as `char::encode_utf16` encodes it, and CodePointAt of the units
    /// reads it back, paired; a surrogate code point is one unit that reads back unpaired; a
    /// value above 0x10FFFF has no encoding.
    #[test]
    fn utf16_encode_code_point_and_code_point_at_round_trip_every_scalar_value() {
        let mut scalars = 0_u32;
        for code_point in 0..=0x10FFFF_u32 {
            let Some(encoded) = utf16_encode_code_point(code_point) else {
                panic!("{code_point:#x} has no encoding");
            };
            let units = encoded.units();
            let read = code_point_at(units, 0);
            match char::from_u32(code_point) {
                Some(scalar) => {
                    scalars += 1;
                    let mut buffer = [0_u16; 2];
                    assert_eq!(units, scalar.encode_utf16(&mut buffer), "{code_point:#x}");
                    assert_eq!(
                        read,
                        Some(CodePoint {
                            code_point,
                            code_unit_count: units.len(),
                            is_unpaired_surrogate: false,
                        }),
                        "{code_point:#x}"
                    );
                }
                None => assert_eq!(
                    read,
                    Some(CodePoint {
                        code_point,
                        code_unit_count: 1,
                        is_unpaired_surrogate: true,
                    }),
                    "{code_point:#x}"
                ),
            }
        }
        assert_eq!(scalars, 0x110000 - 0x800);
        assert!(utf16_encode_code_point(0x110000).is_none());
        assert!(utf16_encode_code_point(u32::MAX).is_none());
    }

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
        let text = agent.string_units_for_tests(text).unwrap();
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
        let text = agent.string_units_for_tests(text).unwrap();
        assert_eq!(String::from_utf16_lossy(&text), "Symbol()");
        let thrown = agent.construct_value(string, &[symbol], None);
        assert!(matches!(thrown, Err(Abrupt::Throw(_))), "{thrown:?}");
    }

    /// The builder of the Strings `join`, `JSON.stringify`, `bind` and `Object.prototype.toString`
    /// make asks the memory budget before it grows (`docs/design/engine.md`, decision 4, "Memory
    /// budget"): with `string_length` far above it, code units whose String would not fit
    /// `memory_bytes` end the run in `Terminated(MemoryBudget)` before the buffer grows, and
    /// nothing more is charged; within the budget they are appended. The builder's 1,000 code
    /// units are charged by its capacity while it lives, and credited when it drops.
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
        assert_eq!(agent.heap.accounting.used(), used + 2000);
        drop(builder);
        assert_eq!(agent.heap.accounting.used(), used);
    }

    /// `String.prototype.concat` (`#sec-string.prototype.concat`) builds its result from other
    /// values, so it is bounded by `string_length` (`docs/design/host-interface.md`, section 2):
    /// with `string_length` 1,000,000, `"a".concat(s)` with s of 1,000,000 code units throws a
    /// `RangeError` before a unit of s is copied.
    #[test]
    fn concat_past_string_length_copies_nothing() {
        use crate::builtins::string::transform::tests::{Answer, Counting, FEW_STEPS, LONG};
        let mut counting = Counting::new(LONG as u64);
        counting.string("s", &vec![0x61; LONG]);
        assert_eq!(
            counting.counted(FEW_STEPS, "\"a\".concat(s)"),
            (Answer::Throw("RangeError".to_string()), 0)
        );
    }
}
