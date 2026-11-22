//! The search and copy methods of `Array` and `Array.prototype` (ECMA-262 23.1.2 and 23.1.3,
//! `#sec-properties-of-the-array-constructor`, `#sec-properties-of-the-array-prototype-object`),
//! the group of `story:array-search-and-copy-methods`: `Array.of` (`#sec-array.of`), `at`
//! (`#sec-array.prototype.at`), `concat` (`#sec-array.prototype.concat`) with
//! IsConcatSpreadable (`#sec-isconcatspreadable`), `includes`
//! (`#sec-array.prototype.includes`), `indexOf` (`#sec-array.prototype.indexof`),
//! `lastIndexOf` (`#sec-array.prototype.lastindexof`), `slice` (`#sec-array.prototype.slice`),
//! `toLocaleString` (`#sec-array.prototype.tolocalestring`, the definition of ECMA-262 without
//! ECMA-402), `toReversed` (`#sec-array.prototype.toreversed`), `toSpliced`
//! (`#sec-array.prototype.tospliced`), `with` (`#sec-array.prototype.with`), and the data
//! property `Array.prototype[%Symbol.unscopables%]`
//! (`#sec-array.prototype-%symbol.unscopables%`), made by the hook of the group for a property
//! keyed by a well-known symbol (`story:builtin-group-modules`).
//!
//! Every loop over indices charges a step per index (decision 2), so a length of 2^32 - 1 or
//! 2^53 - 1 ends in `Terminated(StepBudget)`; the String `toLocaleString` builds goes through
//! the builder of `builtins/string/mod.rs`, bounded by `string_length`. What a builtin holds
//! across code it may run (a getter, a `valueOf`, a species constructor, an element's
//! `toLocaleString`) is on the agent's kept stack: the object it reads, the object it fills and
//! the element it invokes; the arguments are held by its activation (decision 4). The key of an
//! index above 2^32 - 2 is an atom, made again after any code ran.
//!
//! ToAbsoluteIndex (`#sec-toabsoluteindex`) and ToClampedIndex (`#sec-toclampedindex`) are the
//! ones of `ops/conversion.rs`.

use crate::builtins::array::MAX_SAFE_LENGTH;
use crate::builtins::object::argument;
use crate::builtins::string::{StringBuilder, units_of};
use crate::builtins::{Method, SymbolData};
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};
use crate::object::ordinary::OrdinaryData;
use crate::object::property::Key;
use crate::realm::intrinsics::Making;
use crate::value::{ObjectRef, Value};

/// The builtins of this group, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SearchBuiltin {
    /// `Array.of ( ...items )` (23.1.2.4).
    Of,
    /// `Array.prototype.at ( index )` (23.1.3.1).
    At,
    /// `Array.prototype.concat ( ...items )` (23.1.3.2).
    Concat,
    /// `Array.prototype.includes ( searchElement [ , fromIndex ] )` (23.1.3.16).
    Includes,
    /// `Array.prototype.indexOf ( searchElement [ , fromIndex ] )` (23.1.3.17).
    IndexOf,
    /// `Array.prototype.lastIndexOf ( searchElement [ , fromIndex ] )` (23.1.3.20).
    LastIndexOf,
    /// `Array.prototype.slice ( start, end )` (23.1.3.28).
    Slice,
    /// `Array.prototype.toLocaleString ( [ reserved1 [ , reserved2 ] ] )` (23.1.3.32).
    ToLocaleString,
    /// `Array.prototype.toReversed ( )` (23.1.3.33).
    ToReversed,
    /// `Array.prototype.toSpliced ( start, skipCount, ...items )` (23.1.3.35).
    ToSpliced,
    /// `Array.prototype.with ( index, value )` (23.1.3.39).
    With,
}

/// The methods this group defines on `%Array%`, in the order of their clauses.
pub(crate) const CONSTRUCTOR_METHODS: &[Method<SearchBuiltin>] = &[("of", 0, SearchBuiltin::Of)];

/// The methods this group defines on `%Array.prototype%`, in the order of their clauses.
pub(crate) const PROTOTYPE_METHODS: &[Method<SearchBuiltin>] = &[
    ("at", 1, SearchBuiltin::At),
    ("concat", 1, SearchBuiltin::Concat),
    ("includes", 1, SearchBuiltin::Includes),
    ("indexOf", 1, SearchBuiltin::IndexOf),
    ("lastIndexOf", 1, SearchBuiltin::LastIndexOf),
    ("slice", 2, SearchBuiltin::Slice),
    ("toLocaleString", 0, SearchBuiltin::ToLocaleString),
    ("toReversed", 0, SearchBuiltin::ToReversed),
    ("toSpliced", 2, SearchBuiltin::ToSpliced),
    ("with", 2, SearchBuiltin::With),
];

/// The names `Array.prototype[%Symbol.unscopables%]` lists, in the order of its steps; those
/// of `copyWithin`, `entries`, `fill`, the `find` methods, `flat`, `flatMap`, `keys`,
/// `toSorted` and `values` are listed before the methods exist, as the clause lists them.
const UNSCOPABLE_NAMES: [&str; 16] = [
    "at",
    "copyWithin",
    "entries",
    "fill",
    "find",
    "findIndex",
    "findLast",
    "findLastIndex",
    "flat",
    "flatMap",
    "includes",
    "keys",
    "toReversed",
    "toSorted",
    "toSpliced",
    "values",
];

/// The data properties keyed by a well-known symbol this group defines on
/// `%Array.prototype%`, after its methods: `[%Symbol.unscopables%]`, { [[Writable]]: false,
/// [[Enumerable]]: false, [[Configurable]]: true }.
pub(crate) fn prototype_symbol_data<H: Host>() -> [SymbolData<H>; 1] {
    [SymbolData {
        symbol: |symbols| symbols.unscopables,
        value: unscopable_list::<H>,
        writable: false,
        enumerable: false,
        configurable: true,
    }]
}

/// The value of `Array.prototype[%Symbol.unscopables%]`
/// (`#sec-array.prototype-%symbol.unscopables%`): OrdinaryObjectCreate(null), then
/// CreateDataPropertyOrThrow of each name with true. It runs while the realm is made, before
/// any code, so nothing can collect the list between its steps.
fn unscopable_list<H: Host>(agent: &mut Agent<H>, _making: Making) -> Completion<Value> {
    // 1. Let unscopableList be OrdinaryObjectCreate(null).
    let list = agent.ordinary_object_create(None, OrdinaryData::Plain)?;
    // 2 to 17. Perform ! CreateDataPropertyOrThrow(unscopableList, name, true).
    for name in UNSCOPABLE_NAMES {
        let key = agent.name_key(name)?;
        agent.create_data_property_or_throw(list, key, Value::Boolean(true))?;
    }
    // 18. Return unscopableList.
    Ok(Value::Object(list))
}

impl<H: Host> Agent<H> {
    /// The steps of the builtins of this group; not inlined, so that the frame of
    /// `run_array_builtin` does not grow with its arms, and each builtin is a function of its
    /// own, so that a re-entry pays only the frame of the one that runs.
    #[inline(never)]
    pub(crate) fn run_array_search_builtin(
        &mut self,
        id: SearchBuiltin,
        _function: ObjectRef,
        this: Value,
        args: &[Value],
        _new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        match id {
            SearchBuiltin::Of => self.array_of(this, args),
            SearchBuiltin::At => self.array_at(this, argument(args, 0)),
            SearchBuiltin::Concat => self.array_concat(this, args),
            SearchBuiltin::Includes => self.array_includes(this, args),
            SearchBuiltin::IndexOf => self.array_index_of(this, args),
            SearchBuiltin::LastIndexOf => self.array_last_index_of(this, args),
            SearchBuiltin::Slice => self.array_slice(this, argument(args, 0), argument(args, 1)),
            SearchBuiltin::ToLocaleString => self.array_to_locale_string(this),
            SearchBuiltin::ToReversed => self.array_to_reversed(this),
            SearchBuiltin::ToSpliced => self.array_to_spliced(this, args),
            SearchBuiltin::With => self.array_with(this, argument(args, 0), argument(args, 1)),
        }
    }

    /// `Array.of ( ...items )` (`#sec-array.of`). The items are held by this builtin's
    /// activation; the array is kept from its creation to the end.
    #[inline(never)]
    fn array_of(&mut self, this: Value, items: &[Value]) -> Completion<Value> {
        let mark = self.kept.len();
        let made = self.array_of_steps(this, items);
        self.release(mark);
        made
    }

    /// The steps of `Array.of`, with what they keep released by the caller.
    fn array_of_steps(&mut self, this: Value, items: &[Value]) -> Completion<Value> {
        // 1, 2. Let length be the number of elements in items; lengthNumber is 𝔽(length).
        let length = items.len() as f64;
        // 3. Let ctor be the this value.
        // 4. If IsConstructor(ctor) is true, let array be ? Construct(ctor, « lengthNumber »);
        let array = match this {
            Value::Object(constructor) if self.has_construct(constructor)? => {
                match self.construct_value(constructor, &[Value::Number(length)], None)? {
                    Value::Object(made) => made,
                    // Construct answers an Object, or throws.
                    _ => return Err(self.fault()),
                }
            }
            // else let array be ? ArrayCreate(length).
            _ => self.array_create(length, None)?,
        };
        self.keep(Value::Object(array));
        // 5, 6. For each k below length: ? CreateDataPropertyOrThrow(array, ! ToString(𝔽(k)),
        // items[k]).
        for (index, &item) in items.iter().enumerate() {
            self.charge_steps(1)?;
            let key = self.index_key(index as f64)?;
            self.create_data_property_or_throw(array, key, item)?;
        }
        // 7. Perform ? Set(array, "length", lengthNumber, true).
        let key = self.name_key("length")?;
        self.set(array, key, Value::Number(length), true)?;
        // 8. Return array.
        Ok(Value::Object(array))
    }

    /// Run `steps` on ToObject of the this value, with that object kept, and release what the
    /// steps kept after it.
    fn with_kept_this(
        &mut self,
        this: Value,
        steps: fn(&mut Self, ObjectRef, &[Value]) -> Completion<Value>,
        args: &[Value],
    ) -> Completion<Value> {
        // 1. Let obj be ? ToObject(this value). It may be new, and the steps after run code.
        let object = self.to_object(this)?;
        let mark = self.keep(Value::Object(object));
        let answer = steps(self, object, args);
        self.release(mark);
        answer
    }

    /// `Array.prototype.at ( index )` (`#sec-array.prototype.at`).
    #[inline(never)]
    fn array_at(&mut self, this: Value, index: Value) -> Completion<Value> {
        self.with_kept_this(this, Self::array_at_steps, &[index])
    }

    /// Steps 2 to 5 of `at`, with obj kept.
    fn array_at_steps(&mut self, object: ObjectRef, args: &[Value]) -> Completion<Value> {
        // 2. Let length be ? LengthOfArrayLike(obj).
        let length = self.length_of_array_like(object)?;
        // 3. Let k be ? ToAbsoluteIndex(index, length).
        let index = self.to_absolute_index(argument(args, 0), length)?;
        // 4. If k < 0 or k ≥ length, return undefined.
        if index < 0.0 || index >= length {
            return Ok(Value::Undefined);
        }
        // 5. Return ? Get(obj, ! ToString(𝔽(k))).
        let key = self.index_key(index)?;
        self.get(object, key)
    }

    /// `Array.prototype.concat ( ...items )` (`#sec-array.prototype.concat`). The items are
    /// held by this builtin's activation.
    #[inline(never)]
    fn array_concat(&mut self, this: Value, items: &[Value]) -> Completion<Value> {
        self.with_kept_this(this, Self::array_concat_steps, items)
    }

    /// Steps 2 to 7 of `concat`, with obj kept; the result is kept once made.
    fn array_concat_steps(&mut self, object: ObjectRef, items: &[Value]) -> Completion<Value> {
        // 2. Let array be ? ArraySpeciesCreate(obj, 0).
        let array = self.array_species_create(object, 0.0)?;
        self.keep(Value::Object(array));
        // 3. Let nextIndex be 0.
        let mut next = 0.0;
        // 4, 5. Prepend obj to items; for each element item of items:
        for item in std::iter::once(Value::Object(object)).chain(items.iter().copied()) {
            // 5.a. Let spreadable be ? IsConcatSpreadable(item).
            match item {
                Value::Object(spread) if self.is_concat_spreadable(spread)? => {
                    next = self.concat_spread(array, spread, next)?;
                }
                // 5.c. Else: item is added as a single item rather than spread.
                _ => {
                    self.charge_steps(1)?;
                    // 5.c.ii. If nextIndex ≥ 2^53 - 1, throw a TypeError exception.
                    if next >= MAX_SAFE_LENGTH {
                        return Err(self.throw_error(
                            ErrorKind::TypeError,
                            "Array.prototype.concat would make a length above 2^53 - 1",
                        ));
                    }
                    // 5.c.iii, 5.c.iv. ? CreateDataPropertyOrThrow(array,
                    // ! ToString(𝔽(nextIndex)), item); nextIndex + 1.
                    let key = self.index_key(next)?;
                    self.create_data_property_or_throw(array, key, item)?;
                    next += 1.0;
                }
            }
        }
        // 6. Perform ? Set(array, "length", 𝔽(nextIndex), true).
        let key = self.name_key("length")?;
        self.set(array, key, Value::Number(next), true)?;
        // 7. Return array.
        Ok(Value::Object(array))
    }

    /// Step 5.b of `concat`: the elements of the spreadable `item` defined on `array` from the
    /// index `next` on, holes skipped; answers the next index. `item` is obj or an argument,
    /// both held.
    fn concat_spread(&mut self, array: ObjectRef, item: ObjectRef, next: f64) -> Completion<f64> {
        let mut next = next;
        // 5.b.i. Let length be ? LengthOfArrayLike(item).
        let length = self.length_of_array_like(item)?;
        // 5.b.ii. If nextIndex + length > 2^53 - 1, throw a TypeError exception.
        if next + length > MAX_SAFE_LENGTH {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "Array.prototype.concat would make a length above 2^53 - 1",
            ));
        }
        // 5.b.iii, 5.b.iv. For each sourceIndex below length.
        let mut index = 0.0;
        while index < length {
            self.charge_steps(1)?;
            // 5.b.iv.1, 5.b.iv.2. propertyKey; let exists be ? HasProperty(item, propertyKey).
            // HasProperty runs no code, so the key still holds for the Get.
            let key = self.index_key(index)?;
            if self.has_property(item, key)? {
                // 5.b.iv.3.a. Let subElement be ? Get(item, propertyKey).
                let element = self.get(item, key)?;
                // 5.b.iv.3.b. ? CreateDataPropertyOrThrow(array, ! ToString(𝔽(nextIndex)),
                // subElement).
                let key = self.index_key(next)?;
                self.create_data_property_or_throw(array, key, element)?;
            }
            // 5.b.iv.4, 5.b.iv.5. nextIndex + 1, sourceIndex + 1.
            next += 1.0;
            index += 1.0;
        }
        Ok(next)
    }

    /// IsConcatSpreadable (`#sec-isconcatspreadable`) of an object (step 1: any other value is
    /// not spreadable).
    fn is_concat_spreadable(&mut self, object: ObjectRef) -> Completion<bool> {
        let Some(symbols) = self.well_known() else {
            return Err(self.fault());
        };
        // 2. Let spreadable be ? Get(obj, %Symbol.isConcatSpreadable%).
        let spreadable = self.get(object, Key::Symbol(symbols.is_concat_spreadable))?;
        // 3. If spreadable is not undefined, return ToBoolean(spreadable).
        if spreadable != Value::Undefined {
            return self.to_boolean(spreadable);
        }
        // 4. Return ? IsArray(obj).
        self.is_array(Value::Object(object))
    }

    /// `Array.prototype.includes ( searchElement [ , fromIndex ] )`
    /// (`#sec-array.prototype.includes`).
    #[inline(never)]
    fn array_includes(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        self.with_kept_this(this, Self::array_includes_steps, args)
    }

    /// Steps 2 to 6 of `includes`, with obj kept. searchElement is held by the activation.
    fn array_includes_steps(&mut self, object: ObjectRef, args: &[Value]) -> Completion<Value> {
        let search = argument(args, 0);
        // 2. Let length be ? LengthOfArrayLike(obj).
        let length = self.length_of_array_like(object)?;
        // 3. If length = 0, return false.
        if length == 0.0 {
            return Ok(Value::Boolean(false));
        }
        // 4. Let k be ? ToClampedIndex(fromIndex, length).
        let mut index = self.to_clamped_index(argument(args, 1), length)?;
        // 5. Repeat, while k < length.
        while index < length {
            self.charge_steps(1)?;
            // 5.a. Let elementK be ? Get(obj, ! ToString(𝔽(k))): a hole reads as undefined.
            let key = self.index_key(index)?;
            let element = self.get(object, key)?;
            // 5.b. If SameValueZero(searchElement, elementK) is true, return true.
            if self.same_value_zero(search, element)? {
                return Ok(Value::Boolean(true));
            }
            index += 1.0;
        }
        // 6. Return false.
        Ok(Value::Boolean(false))
    }

    /// `Array.prototype.indexOf ( searchElement [ , fromIndex ] )`
    /// (`#sec-array.prototype.indexof`).
    #[inline(never)]
    fn array_index_of(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        self.with_kept_this(this, Self::array_index_of_steps, args)
    }

    /// Steps 2 to 6 of `indexOf`, with obj kept. searchElement is held by the activation.
    fn array_index_of_steps(&mut self, object: ObjectRef, args: &[Value]) -> Completion<Value> {
        let search = argument(args, 0);
        // 2. Let length be ? LengthOfArrayLike(obj).
        let length = self.length_of_array_like(object)?;
        // 3. If length = 0, return -1𝔽.
        if length == 0.0 {
            return Ok(Value::Number(-1.0));
        }
        // 4. Let k be ? ToClampedIndex(fromIndex, length).
        let mut index = self.to_clamped_index(argument(args, 1), length)?;
        // 5. Repeat, while k < length.
        while index < length {
            self.charge_steps(1)?;
            if self.strictly_equal_element(object, index, search)? {
                return Ok(Value::Number(index));
            }
            index += 1.0;
        }
        // 6. Return -1𝔽.
        Ok(Value::Number(-1.0))
    }

    /// The body of the loops of `indexOf` and `lastIndexOf`: whether obj has the index `index`
    /// (HasProperty: a hole is skipped) and its element IsStrictlyEqual `search`.
    fn strictly_equal_element(
        &mut self,
        object: ObjectRef,
        index: f64,
        search: Value,
    ) -> Completion<bool> {
        // 5.a, 5.b. Let propertyKey be ! ToString(𝔽(k)); let kPresent be ? HasProperty(obj,
        // propertyKey). HasProperty runs no code, so the key still holds for the Get.
        let key = self.index_key(index)?;
        if !self.has_property(object, key)? {
            return Ok(false);
        }
        // 5.c.i. Let elementK be ? Get(obj, propertyKey).
        let element = self.get(object, key)?;
        // 5.c.ii. If IsStrictlyEqual(searchElement, elementK) is true, return 𝔽(k).
        self.is_strictly_equal(search, element)
    }

    /// `Array.prototype.lastIndexOf ( searchElement [ , fromIndex ] )`
    /// (`#sec-array.prototype.lastindexof`).
    #[inline(never)]
    fn array_last_index_of(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        self.with_kept_this(this, Self::array_last_index_of_steps, args)
    }

    /// Steps 2 to 6 of `lastIndexOf`, with obj kept. searchElement is held by the activation.
    fn array_last_index_of_steps(
        &mut self,
        object: ObjectRef,
        args: &[Value],
    ) -> Completion<Value> {
        let search = argument(args, 0);
        // 2. Let length be ? LengthOfArrayLike(obj).
        let length = self.length_of_array_like(object)?;
        // 3. If length = 0, return -1𝔽.
        if length == 0.0 {
            return Ok(Value::Number(-1.0));
        }
        // 4. If fromIndex is not present, let k be length - 1; else let k be
        // min(? ToAbsoluteIndex(fromIndex, length), length - 1). An undefined fromIndex is present.
        let mut index = match args.get(1) {
            Some(&from) => self.to_absolute_index(from, length)?.min(length - 1.0),
            None => length - 1.0,
        };
        // 5. Repeat, while k ≥ 0.
        while index >= 0.0 {
            self.charge_steps(1)?;
            if self.strictly_equal_element(object, index, search)? {
                return Ok(Value::Number(index));
            }
            index -= 1.0;
        }
        // 6. Return -1𝔽.
        Ok(Value::Number(-1.0))
    }

    /// `Array.prototype.slice ( start, end )` (`#sec-array.prototype.slice`).
    #[inline(never)]
    fn array_slice(&mut self, this: Value, start: Value, end: Value) -> Completion<Value> {
        self.with_kept_this(this, Self::array_slice_steps, &[start, end])
    }

    /// Steps 2 to 10 of `slice`, with obj kept; the result is kept once made. start and
    /// end are held by the activation.
    fn array_slice_steps(&mut self, object: ObjectRef, args: &[Value]) -> Completion<Value> {
        // 2. Let length be ? LengthOfArrayLike(obj).
        let length = self.length_of_array_like(object)?;
        // 3. Let k be ? ToClampedIndex(start, length).
        let mut index = self.to_clamped_index(argument(args, 0), length)?;
        // 4. If end is undefined, let final be length; else ? ToClampedIndex(end, length).
        let end = match argument(args, 1) {
            Value::Undefined => length,
            end => self.to_clamped_index(end, length)?,
        };
        // 5. Let count be max(final - k, 0).
        let count = (end - index).max(0.0);
        // 6. Let array be ? ArraySpeciesCreate(obj, count).
        let array = self.array_species_create(object, count)?;
        self.keep(Value::Object(array));
        // 7. Let resultIndex be 0.
        let mut next = 0.0;
        // 8. Repeat, while k < final.
        while index < end {
            self.charge_steps(1)?;
            // 8.a, 8.b. Let propertyKey be ! ToString(𝔽(k)); let kPresent be
            // ? HasProperty(obj, propertyKey). HasProperty runs no code, so the key still holds
            // for the Get.
            let key = self.index_key(index)?;
            if self.has_property(object, key)? {
                // 8.c.i. Let kValue be ? Get(obj, propertyKey).
                let value = self.get(object, key)?;
                // 8.c.ii. ? CreateDataPropertyOrThrow(array, ! ToString(𝔽(resultIndex)), kValue).
                let key = self.index_key(next)?;
                self.create_data_property_or_throw(array, key, value)?;
            }
            // 8.d, 8.e. k + 1, resultIndex + 1.
            index += 1.0;
            next += 1.0;
        }
        // 9. Perform ? Set(array, "length", 𝔽(resultIndex), true).
        let key = self.name_key("length")?;
        self.set(array, key, Value::Number(next), true)?;
        // 10. Return array.
        Ok(Value::Object(array))
    }

    /// `Array.prototype.toLocaleString ( [ reserved1 [ , reserved2 ] ] )`
    /// (`#sec-array.prototype.tolocalestring`), as ECMA-262 defines it for an implementation
    /// without ECMA-402: the arguments are not used.
    #[inline(never)]
    fn array_to_locale_string(&mut self, this: Value) -> Completion<Value> {
        self.with_kept_this(this, Self::array_to_locale_string_steps, &[])
    }

    /// Steps 2 and 3 of `toLocaleString`, with the array kept: JoinIndexedProperties
    /// (`#sec-joinindexedproperties`) with the methodName "toLocaleString" and no methodArgs.
    /// LocalizedListSeparator (`#sec-localizedlistseparator`) is implementation-defined; it is
    /// "," here. A step per index; the result is bounded by `string_length` before it grows.
    fn array_to_locale_string_steps(
        &mut self,
        array: ObjectRef,
        _args: &[Value],
    ) -> Completion<Value> {
        // 2. Let length be ? LengthOfArrayLike(array).
        let length = self.length_of_array_like(array)?;
        let separator = units_of(",");
        let mut result = StringBuilder::new();
        let mut index = 0.0;
        while index < length {
            self.charge_steps(1)?;
            // 4.a. If k > 0, set result to the string-concatenation of result and separator.
            if index > 0.0 {
                self.append_units(&mut result, &separator)?;
            }
            // 4.c. Let element be ? Get(array, ! ToString(𝔽(k))).
            let key = self.index_key(index)?;
            let element = self.get(array, key)?;
            // 4.d. If element is neither undefined nor null, then
            if !matches!(element, Value::Undefined | Value::Null) {
                self.append_locale_string(&mut result, element)?;
            }
            index += 1.0;
        }
        // 3. Return ? JoinIndexedProperties(...), whose step 5 returns result.
        let joined = self.finish_string(result)?;
        Ok(Value::String(joined))
    }

    /// Step 4.d.i.1 of JoinIndexedProperties with a methodName and an empty methodArgs: append
    /// ? ToString(? Invoke(element, "toLocaleString", « »)) (`#sec-invoke`). The element is kept
    /// across the Get of its method and the call.
    fn append_locale_string(
        &mut self,
        result: &mut StringBuilder,
        element: Value,
    ) -> Completion<()> {
        let mark = self.keep(element);
        let appended = self.append_locale_string_steps(result, element);
        self.release(mark);
        appended
    }

    /// The steps of [`Agent::append_locale_string`], with the element kept.
    fn append_locale_string_steps(
        &mut self,
        result: &mut StringBuilder,
        element: Value,
    ) -> Completion<()> {
        let key = self.name_key("toLocaleString")?;
        let answer = self.invoke(element, key, &[])?;
        // ToString may call the answer's own methods, which hold it as their this value.
        let string = self.to_string(answer)?;
        self.append_string(result, string)
    }

    /// `Array.prototype.toReversed ( )` (`#sec-array.prototype.toreversed`).
    #[inline(never)]
    fn array_to_reversed(&mut self, this: Value) -> Completion<Value> {
        self.with_kept_this(this, Self::array_to_reversed_steps, &[])
    }

    /// Steps 2 to 6 of `toReversed`, with obj kept; the result is kept once made.
    fn array_to_reversed_steps(&mut self, object: ObjectRef, _args: &[Value]) -> Completion<Value> {
        // 2. Let length be ? LengthOfArrayLike(obj).
        let length = self.length_of_array_like(object)?;
        // 3. Let array be ? ArrayCreate(length): a RangeError above 2^32 - 1, before any Get.
        let array = self.array_create(length, None)?;
        self.keep(Value::Object(array));
        // 4, 5. For each k below length.
        let mut index = 0.0;
        while index < length {
            self.charge_steps(1)?;
            // 5.a, 5.c. Let from be ! ToString(𝔽(length - k - 1)); fromValue is ? Get(obj, from).
            let from = self.index_key(length - index - 1.0)?;
            let value = self.get(object, from)?;
            // 5.b, 5.d. propertyKey is ! ToString(𝔽(k)); ! CreateDataPropertyOrThrow(array, ...).
            let key = self.index_key(index)?;
            self.create_data_property_or_throw(array, key, value)?;
            index += 1.0;
        }
        // 6. Return array.
        Ok(Value::Object(array))
    }

    /// `Array.prototype.toSpliced ( start, skipCount, ...items )`
    /// (`#sec-array.prototype.tospliced`). Whether start and skipCount are present is read
    /// from the count of the arguments; the items are held by this builtin's activation.
    #[inline(never)]
    fn array_to_spliced(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        self.with_kept_this(this, Self::array_to_spliced_steps, args)
    }

    /// Steps 2 to 16 of `toSpliced`, with obj kept; the result is kept once made.
    fn array_to_spliced_steps(&mut self, object: ObjectRef, args: &[Value]) -> Completion<Value> {
        // 2. Let length be ? LengthOfArrayLike(obj).
        let length = self.length_of_array_like(object)?;
        // 3. Let actualStart be ? ToClampedIndex(start, length).
        let start = self.to_clamped_index(argument(args, 0), length)?;
        // 4. Let insertCount be the number of elements in items.
        let items = args.get(2..).unwrap_or_default();
        let insert = items.len() as f64;
        // 5, 6. actualSkipCount: 0 without start, maxSkipCount (length - actualStart) without
        // skipCount, otherwise ? ToIntegerOrInfinity(skipCount) clamped to 0 and maxSkipCount.
        let most = length - start;
        let skip = match args {
            [] => 0.0,
            [_] => most,
            [_, skip, ..] => self.to_integer_or_infinity(*skip)?.clamp(0.0, most),
        };
        // 7, 8. Let newLength be length + insertCount - actualSkipCount, at least 0.
        let new_length = length + insert - skip;
        // 9. If newLength > 2^53 - 1, throw a TypeError exception.
        if new_length > MAX_SAFE_LENGTH {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "Array.prototype.toSpliced would make a length above 2^53 - 1",
            ));
        }
        // 10. Let newArray be ? ArrayCreate(newLength): a RangeError above 2^32 - 1, before a Get.
        let array = self.array_create(new_length, None)?;
        self.keep(Value::Object(array));
        // 11, 12. Let writeIndex be 0; let readIndex be actualStart + actualSkipCount.
        let mut write = 0.0;
        let mut read = start + skip;
        // 13. Repeat, while writeIndex < actualStart: ? Get, ! CreateDataPropertyOrThrow.
        while write < start {
            self.charge_steps(1)?;
            let key = self.index_key(write)?;
            let value = self.get(object, key)?;
            let key = self.index_key(write)?;
            self.create_data_property_or_throw(array, key, value)?;
            write += 1.0;
        }
        // 14. For each element item of items: ! CreateDataPropertyOrThrow; writeIndex + 1.
        for &item in items {
            self.charge_steps(1)?;
            let key = self.index_key(write)?;
            self.create_data_property_or_throw(array, key, item)?;
            write += 1.0;
        }
        // 15. Repeat, while writeIndex < newLength: ? Get(obj, ! ToString(𝔽(readIndex))), then
        // ! CreateDataPropertyOrThrow at writeIndex; writeIndex + 1, readIndex + 1.
        while write < new_length {
            self.charge_steps(1)?;
            let from = self.index_key(read)?;
            let value = self.get(object, from)?;
            let key = self.index_key(write)?;
            self.create_data_property_or_throw(array, key, value)?;
            write += 1.0;
            read += 1.0;
        }
        // 16. Return newArray.
        Ok(Value::Object(array))
    }

    /// `Array.prototype.with ( index, value )` (`#sec-array.prototype.with`).
    #[inline(never)]
    fn array_with(&mut self, this: Value, index: Value, value: Value) -> Completion<Value> {
        self.with_kept_this(this, Self::array_with_steps, &[index, value])
    }

    /// Steps 2 to 8 of `with`, with obj kept; the result is kept once made. value is
    /// held by the activation.
    fn array_with_steps(&mut self, object: ObjectRef, args: &[Value]) -> Completion<Value> {
        let replacement = argument(args, 1);
        // 2. Let length be ? LengthOfArrayLike(obj).
        let length = self.length_of_array_like(object)?;
        // 3. Let actualIndex be ? ToAbsoluteIndex(index, length).
        let replaced = self.to_absolute_index(argument(args, 0), length)?;
        // 4. If actualIndex < 0 or actualIndex ≥ length, throw a RangeError exception.
        if replaced < 0.0 || replaced >= length {
            return Err(self.throw_error(
                ErrorKind::RangeError,
                "Array.prototype.with: the index is outside the array",
            ));
        }
        // 5. Let array be ? ArrayCreate(length): a RangeError above 2^32 - 1, before any Get.
        let array = self.array_create(length, None)?;
        self.keep(Value::Object(array));
        // 6, 7. For each k below length.
        let mut index = 0.0;
        while index < length {
            self.charge_steps(1)?;
            // 7.a to 7.c. Let propertyKey be ! ToString(𝔽(k)); fromValue is value at
            // actualIndex, ? Get(obj, propertyKey) elsewhere.
            let value = if index == replaced {
                replacement
            } else {
                let key = self.index_key(index)?;
                self.get(object, key)?
            };
            // 7.d. Perform ! CreateDataPropertyOrThrow(array, propertyKey, fromValue).
            let key = self.index_key(index)?;
            self.create_data_property_or_throw(array, key, value)?;
            index += 1.0;
        }
        // 8. Return array.
        Ok(Value::Object(array))
    }
}

#[cfg(test)]
mod tests {
    use crate::host::ErrorKind;
    use crate::host::agent::Agent;
    use crate::host::hooks::test_host::{NoHooks, agent_with_realm, enter};
    use crate::host::outcome::{Abrupt, Completion};
    use crate::host::{Cx, HostCall};
    use crate::object::ordinary::OrdinaryData;
    use crate::object::property::{Descriptor, Key, Property, Slot};
    use crate::realm::intrinsics::Intrinsic;
    use crate::value::{ObjectRef, Value};

    /// The method `name` of `%Array.prototype%` of the entered realm, kept.
    fn method(agent: &mut Agent<NoHooks>, name: &str) -> Value {
        let prototype = agent.current_intrinsic(Intrinsic::ArrayPrototype).unwrap();
        let key = agent.name_key(name).unwrap();
        let method = agent.get(prototype, key).unwrap();
        agent.keep(method);
        method
    }

    /// A new ordinary object of the entered realm, kept.
    fn plain(agent: &mut Agent<NoHooks>) -> ObjectRef {
        let prototype = agent.current_intrinsic(Intrinsic::ObjectPrototype).unwrap();
        let object = agent
            .ordinary_object_create(Some(prototype), OrdinaryData::Plain)
            .unwrap();
        agent.keep(Value::Object(object));
        object
    }

    /// A new Array of the entered realm with `elements` (`None` a hole), kept.
    fn array_of(agent: &mut Agent<NoHooks>, elements: &[Option<Value>]) -> ObjectRef {
        let array = agent.array_create(elements.len() as f64, None).unwrap();
        agent.keep(Value::Object(array));
        for (index, element) in elements.iter().enumerate() {
            if let Some(element) = *element {
                let index = u32::try_from(index).unwrap();
                agent
                    .create_data_property(array, Key::Index(index), element)
                    .unwrap();
            }
        }
        array
    }

    /// The "length" of `object`.
    fn length(agent: &mut Agent<NoHooks>, object: ObjectRef) -> Value {
        let key = agent.name_key("length").unwrap();
        agent.get(object, key).unwrap()
    }

    /// Whether a completion is a throw of the `kind` error of the entered realm.
    fn throws(agent: &mut Agent<NoHooks>, completion: Completion<Value>, kind: ErrorKind) -> bool {
        let Err(Abrupt::Throw(Value::Object(error))) = completion else {
            return false;
        };
        let prototype = agent
            .current_intrinsic(Intrinsic::ErrorPrototype(kind))
            .unwrap();
        agent.get_prototype_of(error) == Ok(Some(prototype))
    }

    /// A species constructor that records the length it is given as its callee's "seen"
    /// property and answers a new ordinary object.
    fn species(cx: &mut Cx<'_, NoHooks>, call: HostCall<'_>) -> Completion<Value> {
        let agent = &mut *cx.agent;
        let key = agent.name_key("seen")?;
        let length = call.args.first().copied().unwrap_or(Value::Undefined);
        agent.create_data_property(call.callee, key, length)?;
        let prototype = agent.current_intrinsic(Intrinsic::ObjectPrototype)?;
        let made = agent.ordinary_object_create(Some(prototype), OrdinaryData::Plain)?;
        Ok(Value::Object(made))
    }

    /// A getter that records its call as its callee's "seen" property and answers 0.
    fn record_read(cx: &mut Cx<'_, NoHooks>, call: HostCall<'_>) -> Completion<Value> {
        let agent = &mut *cx.agent;
        let key = agent.name_key("seen")?;
        agent.create_data_property(call.callee, key, Value::Boolean(true))?;
        Ok(Value::Number(0.0))
    }

    /// `#sec-array.prototype.slice`, step 6, and `#sec-array.prototype.concat`, step 2:
    /// ArraySpeciesCreate (`#sec-arrayspeciescreate`), the part that needs @@species, which no
    /// script reaches before slice 6. An Array whose constructor has an own @@species
    /// constructor gets its result from that constructor: `slice` constructs it with the count
    /// and `concat` with 0, both fill it (holes skipped) and set its "length"; a @@species of
    /// null makes an Array; one that is not a constructor throws a TypeError.
    #[test]
    fn slice_and_concat_use_the_species() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let species_key = Key::Symbol(agent.well_known().unwrap().species);
        let constructor = agent
            .create_host_function(handle, "Species", 0, species, true, None)
            .unwrap();
        let owner = agent
            .create_host_function(handle, "Owner", 0, species, true, None)
            .unwrap();
        agent.keep(Value::Object(constructor));
        agent.keep(Value::Object(owner));
        let a = Value::String(agent.new_string_str("a").unwrap());
        agent.keep(a);
        let source = array_of(
            &mut agent,
            &[
                Some(a),
                None,
                Some(Value::Number(2.0)),
                Some(Value::Number(3.0)),
            ],
        );
        let constructor_key = agent.name_key("constructor").unwrap();
        agent
            .create_data_property(source, constructor_key, Value::Object(owner))
            .unwrap();
        agent
            .create_data_property(owner, species_key, Value::Object(constructor))
            .unwrap();
        let slice = method(&mut agent, "slice");
        let concat = method(&mut agent, "concat");

        let range = [Value::Number(0.0), Value::Number(3.0)];
        let Ok(Value::Object(sliced)) = agent.call_value(slice, Value::Object(source), &range)
        else {
            panic!("slice answered no object");
        };
        agent.keep(Value::Object(sliced));
        let seen = agent.name_key("seen").unwrap();
        assert_eq!(agent.get(constructor, seen), Ok(Value::Number(3.0)));
        assert_eq!(agent.is_array(Value::Object(sliced)), Ok(false));
        assert_eq!(agent.get(sliced, Key::Index(0)), Ok(a));
        assert_eq!(agent.has_property(sliced, Key::Index(1)), Ok(false));
        assert_eq!(agent.get(sliced, Key::Index(2)), Ok(Value::Number(2.0)));
        assert_eq!(agent.has_property(sliced, Key::Index(3)), Ok(false));
        assert_eq!(length(&mut agent, sliced), Value::Number(3.0));

        let Ok(Value::Object(joined)) =
            agent.call_value(concat, Value::Object(source), &[Value::Number(5.0)])
        else {
            panic!("concat answered no object");
        };
        agent.keep(Value::Object(joined));
        let seen = agent.name_key("seen").unwrap();
        assert_eq!(agent.get(constructor, seen), Ok(Value::Number(0.0)));
        assert_eq!(agent.is_array(Value::Object(joined)), Ok(false));
        assert_eq!(agent.get(joined, Key::Index(0)), Ok(a));
        assert_eq!(agent.has_property(joined, Key::Index(1)), Ok(false));
        assert_eq!(agent.get(joined, Key::Index(3)), Ok(Value::Number(3.0)));
        assert_eq!(agent.get(joined, Key::Index(4)), Ok(Value::Number(5.0)));
        assert_eq!(length(&mut agent, joined), Value::Number(5.0));

        // A @@species of null makes an Array; one that is not a constructor throws.
        let null_species = Descriptor::data(Value::Null, true, true, true);
        agent
            .define_own_property(owner, species_key, null_species)
            .unwrap();
        for (function, args) in [(slice, &range[..]), (concat, &[][..])] {
            let Ok(Value::Object(made)) = agent.call_value(function, Value::Object(source), args)
            else {
                panic!("no object made");
            };
            assert_eq!(agent.is_array(Value::Object(made)), Ok(true));
        }
        let not_a_constructor = Descriptor::data(slice, true, true, true);
        agent
            .define_own_property(owner, species_key, not_a_constructor)
            .unwrap();
        for (function, args) in [(slice, &range[..]), (concat, &[][..])] {
            let made = agent.call_value(function, Value::Object(source), args);
            assert!(throws(&mut agent, made, ErrorKind::TypeError));
        }
    }

    /// IsConcatSpreadable (`#sec-isconcatspreadable`) reads @@isConcatSpreadable, which no
    /// script can set before slice 6: an array-like with a truthy one is spread, holes kept, an
    /// Array with a falsy one is one element, and without one an Array is spread and an
    /// array-like is not. A spreadable object whose length would take the result above
    /// 2^53 - 1 throws a TypeError before any of its elements is read
    /// (`#sec-array.prototype.concat`, step 5.b.ii).
    #[test]
    fn concat_spreads_by_is_concat_spreadable_and_refuses_a_length_above_2_53() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let spreadable = Key::Symbol(agent.well_known().unwrap().is_concat_spreadable);
        let concat = method(&mut agent, "concat");
        let length_key = agent.name_key("length").unwrap();

        let like = plain(&mut agent);
        for (key, value) in [
            (length_key, Value::Number(3.0)),
            (Key::Index(0), Value::Number(10.0)),
            (Key::Index(2), Value::Number(12.0)),
            (spreadable, Value::Number(1.0)),
        ] {
            agent.create_data_property(like, key, value).unwrap();
        }
        let plain_like = plain(&mut agent);
        for (key, value) in [
            (length_key, Value::Number(1.0)),
            (Key::Index(0), Value::Number(30.0)),
        ] {
            agent.create_data_property(plain_like, key, value).unwrap();
        }
        let closed = array_of(&mut agent, &[Some(Value::Number(20.0))]);
        agent
            .create_data_property(closed, spreadable, Value::Number(0.0))
            .unwrap();
        let open = array_of(&mut agent, &[Some(Value::Number(40.0))]);
        let receiver = array_of(&mut agent, &[Some(Value::Number(1.0))]);

        let items = [
            Value::Object(like),
            Value::Object(closed),
            Value::Object(plain_like),
            Value::Object(open),
        ];
        let Ok(Value::Object(joined)) = agent.call_value(concat, Value::Object(receiver), &items)
        else {
            panic!("concat answered no object");
        };
        agent.keep(Value::Object(joined));
        assert_eq!(length(&mut agent, joined), Value::Number(7.0));
        let expected = [
            Some(Value::Number(1.0)),
            Some(Value::Number(10.0)),
            None,
            Some(Value::Number(12.0)),
            Some(Value::Object(closed)),
            Some(Value::Object(plain_like)),
            Some(Value::Number(40.0)),
        ];
        for (index, element) in expected.into_iter().enumerate() {
            let key = Key::Index(u32::try_from(index).unwrap());
            match element {
                Some(value) => assert_eq!(agent.get(joined, key), Ok(value), "{index}"),
                None => assert_eq!(agent.has_property(joined, key), Ok(false), "{index}"),
            }
        }

        // 1 + (2^53 - 1) > 2^53 - 1: a TypeError, and the getter of index 0 is never called.
        let getter = agent
            .create_host_function(handle, "read", 0, record_read, false, None)
            .unwrap();
        agent.keep(Value::Object(getter));
        let huge = plain(&mut agent);
        let length_key = agent.name_key("length").unwrap();
        agent
            .create_data_property(huge, length_key, Value::Number(9_007_199_254_740_991.0))
            .unwrap();
        agent
            .create_data_property(huge, spreadable, Value::Boolean(true))
            .unwrap();
        agent
            .define_own_property(
                huge,
                Key::Index(0),
                Descriptor {
                    get: Some(Some(getter)),
                    set: Some(None),
                    enumerable: Some(true),
                    configurable: Some(true),
                    ..Descriptor::default()
                },
            )
            .unwrap();
        let refused = agent.call_value(concat, Value::Object(receiver), &[Value::Object(huge)]);
        assert!(throws(&mut agent, refused, ErrorKind::TypeError));
        let seen = agent.name_key("seen").unwrap();
        assert_eq!(agent.has_own_property(getter, seen), Ok(false));
    }

    /// `#sec-array.prototype-%symbol.unscopables%`: `Array.prototype[@@unscopables]` is a data
    /// property { [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: true } whose
    /// value is an object with a null prototype and the sixteen names of the clause as its own
    /// keys, in the clause's order, each a data property true made by
    /// CreateDataPropertyOrThrow.
    #[test]
    fn array_prototype_unscopables_lists_the_sixteen_names() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let prototype = agent.intrinsic(handle, Intrinsic::ArrayPrototype).unwrap();
        let unscopables = Key::Symbol(agent.well_known().unwrap().unscopables);
        let property = agent
            .get_own_property(prototype, unscopables)
            .unwrap()
            .unwrap();
        let Slot::Data {
            value: Value::Object(list),
            writable: false,
        } = property.slot
        else {
            panic!("{property:?}");
        };
        assert!(!property.enumerable && property.configurable);
        assert_eq!(agent.get_prototype_of(list), Ok(None));
        let names = [
            "at",
            "copyWithin",
            "entries",
            "fill",
            "find",
            "findIndex",
            "findLast",
            "findLastIndex",
            "flat",
            "flatMap",
            "includes",
            "keys",
            "toReversed",
            "toSorted",
            "toSpliced",
            "values",
        ];
        let keys: Vec<Key> = names
            .iter()
            .map(|name| agent.name_key(name).unwrap())
            .collect();
        assert_eq!(
            agent.own_property_keys(list).map(|own| own.keys),
            Ok(keys.clone())
        );
        for key in keys {
            assert_eq!(
                agent.get_own_property(list, key),
                Ok(Some(Property::data(Value::Boolean(true), true, true, true)))
            );
        }
    }
}
