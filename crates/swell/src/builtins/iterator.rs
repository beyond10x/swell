//! The iterator objects of slice 6, written by
//! `story:iterator-protocol-and-array-string-iterators`: `%Iterator.prototype%`
//! (`#sec-%iteratorprototype%-object`) with its `[%Symbol.iterator%]`
//! (`#sec-%iteratorprototype%-%symbol.iterator%`); the Array Iterators (23.1.5,
//! `#sec-%arrayiteratorprototype%-object`) with CreateArrayIterator (`#sec-createarrayiterator`),
//! `%ArrayIteratorPrototype%.next` (`#sec-%arrayiteratorprototype%.next`) and its
//! `[%Symbol.toStringTag%]` (`#sec-%arrayiteratorprototype%-%symbol.tostringtag%`), made by
//! `Array.prototype.values`, `keys` and `entries` (`builtins/array/mod.rs`) and by the
//! `[%Symbol.iterator%]` of an arguments object (`object/arguments.rs`); and the String
//! Iterators (22.1.5, `#sec-%stringiteratorprototype%-object`) with CreateStringIterator
//! (`#sec-createstringiterator`), `%StringIteratorPrototype%.next`
//! (`#sec-%stringiteratorprototype%.next`) and its `[%Symbol.toStringTag%]`
//! (`#sec-%stringiteratorprototype%-%symbol.tostringtag%`), made by
//! `String.prototype[%Symbol.iterator%]` (`builtins/string/mod.rs`).
//!
//! The other properties of `%Iterator.prototype%` and the `%Iterator%` constructor are slice
//! 15. The pinned draft states the String Iterator as a generator made by
//! CreateIteratorFromClosure, whose `next` is GeneratorResume with the brand
//! "%StringIteratorPrototype%"; generators are slice 7, so an iterator here holds the state the
//! closure captures in slots of its own (`OrdinaryData::StringIterator`: the String and the
//! position), and `next` does what a resumption does: it throws a `TypeError` for a this value
//! that is not a String Iterator (GeneratorValidate), answers each code point, and answers done
//! from the end on.
//!
//! An iterator holds what it iterates (`docs/design/engine.md`, decision 3: the domain
//! relations "Array Iterator → the iterated object" and "String Iterator → the iterated
//! string") until `next` finds the end, then drops it; the collector traces the slots
//! (`object/ordinary.rs`). An iterator object is charged to the memory budget when it is
//! made, as every object is, and each `next` is charged its step by the call path before it
//! runs (`object/builtin_function.rs`).

use crate::builtins::string::code_point_at;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};
use crate::object::ObjectKind;
use crate::object::builtin_function::BuiltinId;
use crate::object::ordinary::OrdinaryData;
use crate::object::property::{Key, Property};
use crate::realm::intrinsics::{Intrinsic, Making};
use crate::value::{ObjectRef, StringRef, Value};

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IteratorBuiltin {
    /// `%Iterator.prototype% [ %Symbol.iterator% ] ( )` (27.1.3.3.18).
    Iterator,
    /// `%ArrayIteratorPrototype%.next ( )` (23.1.5.2.1).
    ArrayIteratorNext,
    /// `%StringIteratorPrototype%.next ( )` (22.1.5.1.1).
    StringIteratorNext,
}

/// [[ArrayLikeIterationKind]] of an Array Iterator (23.1.5.3, Table 70).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ArrayIterationKind {
    /// key: the index.
    Key,
    /// value: the element.
    Value,
    /// key+value: an Array of the index and the element.
    KeyValue,
}

/// `%Iterator.prototype%`, `%ArrayIteratorPrototype%` and `%StringIteratorPrototype%` of a realm.
#[derive(Clone, Copy, Debug)]
pub(crate) struct IteratorPrototypes {
    pub(crate) iterator: ObjectRef,
    pub(crate) array_iterator: ObjectRef,
    pub(crate) string_iterator: ObjectRef,
}

impl<H: Host> Agent<H> {
    /// `%Iterator.prototype%`, an ordinary object whose [[Prototype]] is `%Object.prototype%`
    /// (`#sec-%iteratorprototype%-object`), with its `[%Symbol.iterator%]` named
    /// "[Symbol.iterator]" of "length" 0; `%ArrayIteratorPrototype%` and
    /// `%StringIteratorPrototype%`, ordinary objects whose [[Prototype]] is `%Iterator.prototype%`,
    /// each with "next" of "length" 0 and its `[%Symbol.toStringTag%]`, "Array Iterator" and
    /// "String Iterator", { [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: true }.
    /// The methods are { [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]: true }
    /// (clause 18).
    pub(crate) fn create_iterator_prototypes(
        &mut self,
        making: Making,
    ) -> Completion<IteratorPrototypes> {
        let iterator =
            self.ordinary_object_create(Some(making.object_prototype), OrdinaryData::Plain)?;
        let answer_this = self.create_builtin(
            making,
            "[Symbol.iterator]",
            0,
            BuiltinId::Iterator(IteratorBuiltin::Iterator),
        )?;
        self.add_property(
            iterator,
            Key::Symbol(making.symbols.iterator),
            Property::data(Value::Object(answer_this), true, false, true),
        )?;
        let array_iterator = self.create_iterator_kind_prototype(
            making,
            iterator,
            IteratorBuiltin::ArrayIteratorNext,
            "Array Iterator",
        )?;
        let string_iterator = self.create_iterator_kind_prototype(
            making,
            iterator,
            IteratorBuiltin::StringIteratorNext,
            "String Iterator",
        )?;
        Ok(IteratorPrototypes {
            iterator,
            array_iterator,
            string_iterator,
        })
    }

    /// An ordinary object whose [[Prototype]] is `iterator`, with "next" of "length" 0 whose
    /// steps are `next`, then its `[%Symbol.toStringTag%]` `tag`.
    fn create_iterator_kind_prototype(
        &mut self,
        making: Making,
        iterator: ObjectRef,
        next: IteratorBuiltin,
        tag: &str,
    ) -> Completion<ObjectRef> {
        let prototype = self.ordinary_object_create(Some(iterator), OrdinaryData::Plain)?;
        self.define_builtin_method(making, prototype, "next", 0, BuiltinId::Iterator(next))?;
        self.define_to_string_tag(making, prototype, tag)?;
        Ok(prototype)
    }

    /// The steps of the builtins of this module.
    pub(crate) fn run_iterator_builtin(
        &mut self,
        id: IteratorBuiltin,
        _function: ObjectRef,
        this: Value,
        _args: &[Value],
        _new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        match id {
            // `#sec-%iteratorprototype%-%symbol.iterator%`: 1. Return the this value.
            IteratorBuiltin::Iterator => Ok(this),
            IteratorBuiltin::ArrayIteratorNext => self.array_iterator_next(this),
            IteratorBuiltin::StringIteratorNext => self.string_iterator_next(this),
        }
    }

    /// `Array.prototype.values ( )` (`#sec-array.prototype.values`), `keys ( )`
    /// (`#sec-array.prototype.keys`) and `entries ( )` (`#sec-array.prototype.entries`), by
    /// `kind`: 1. Let O be ? ToObject(this value). 2. Return CreateArrayIterator(O, kind). No
    /// code runs between the two steps.
    pub(crate) fn array_iterator_of(
        &mut self,
        this: Value,
        kind: ArrayIterationKind,
    ) -> Completion<Value> {
        let object = self.to_object(this)?;
        Ok(Value::Object(self.create_array_iterator(object, kind)?))
    }

    /// CreateArrayIterator ( array, kind ) (`#sec-createarrayiterator`): an ordinary object of
    /// `%ArrayIteratorPrototype%` of the current realm with [[IteratedArrayLike]] `array`,
    /// [[ArrayLikeNextIndex]] 0 and [[ArrayLikeIterationKind]] `kind`.
    pub(crate) fn create_array_iterator(
        &mut self,
        array: ObjectRef,
        kind: ArrayIterationKind,
    ) -> Completion<ObjectRef> {
        let prototype = self.current_intrinsic(Intrinsic::ArrayIteratorPrototype)?;
        self.ordinary_object_create(
            Some(prototype),
            OrdinaryData::ArrayIterator {
                iterated: Some(array),
                next_index: 0.0,
                kind,
            },
        )
    }

    /// `String.prototype [ %Symbol.iterator% ] ( )` (`#sec-string.prototype-%symbol.iterator%`):
    /// steps 1 to 3, RequireObjectCoercible and ToString of the this value, then steps 4 to 6,
    /// the iterator over the code points of the String ([`Agent::create_string_iterator`]). No
    /// code runs after ToString.
    pub(crate) fn string_iterator_of(&mut self, this: Value) -> Completion<Value> {
        let string = self.this_string(this, "String.prototype[Symbol.iterator]")?;
        Ok(Value::Object(self.create_string_iterator(string)?))
    }

    /// CreateStringIterator (`#sec-createstringiterator`; in the pinned draft
    /// CreateIteratorFromClosure with `%StringIteratorPrototype%`): an ordinary object of
    /// `%StringIteratorPrototype%` of the current realm that holds `string` and the position 0.
    pub(crate) fn create_string_iterator(&mut self, string: StringRef) -> Completion<ObjectRef> {
        let prototype = self.current_intrinsic(Intrinsic::StringIteratorPrototype)?;
        self.ordinary_object_create(
            Some(prototype),
            OrdinaryData::StringIterator {
                iterated: Some(string),
                position: 0,
            },
        )
    }

    /// `%ArrayIteratorPrototype%.next ( )` (`#sec-%arrayiteratorprototype%.next`). The array is
    /// kept while the steps run: a getter they call may run `next` of the same iterator, which
    /// can set its [[IteratedArrayLike]] to undefined and leave the array held by nothing else.
    fn array_iterator_next(&mut self, this: Value) -> Completion<Value> {
        // 1. Let iteratorObj be the this value. 2. If iteratorObj is not an Object, throw a
        // TypeError exception. 3. If iteratorObj does not have all of the internal slots of an
        // Array Iterator Instance (23.1.5.3), throw a TypeError exception.
        let slots = match this {
            Value::Object(iterator) => match &self.engine_object(iterator)?.kind {
                ObjectKind::Ordinary(ordinary) => match ordinary.data {
                    OrdinaryData::ArrayIterator {
                        iterated,
                        next_index,
                        kind,
                    } => Some((iterator, iterated, next_index, kind)),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        };
        let Some((iterator, iterated, index, kind)) = slots else {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "%ArrayIteratorPrototype%.next requires that 'this' be an Array Iterator",
            ));
        };
        // 4. Let array be iteratorObj.[[IteratedArrayLike]]. 5. If array is undefined, return
        // CreateIteratorResultObject(undefined, true).
        let Some(array) = iterated else {
            return Ok(Value::Object(
                self.create_iter_result_object(Value::Undefined, true)?,
            ));
        };
        let mark = self.keep(Value::Object(array));
        let answer = self.array_iterator_next_steps(iterator, array, index, kind);
        self.release(mark);
        answer
    }

    /// Steps 6 to 15 of `%ArrayIteratorPrototype%.next`, with the array kept and the iterator
    /// held by the activation as its this value. A typed array (step 8) is slice 13.
    fn array_iterator_next_steps(
        &mut self,
        iterator: ObjectRef,
        array: ObjectRef,
        index: f64,
        kind: ArrayIterationKind,
    ) -> Completion<Value> {
        // 6. Let index be iteratorObj.[[ArrayLikeNextIndex]]. 7. Let kind be
        // iteratorObj.[[ArrayLikeIterationKind]]. 9. Else, let length be ?
        // LengthOfArrayLike(array).
        let length = self.length_of_array_like(array)?;
        // 10. If index ≥ length, then set iteratorObj.[[IteratedArrayLike]] to undefined and
        // return CreateIteratorResultObject(undefined, true).
        if index >= length {
            self.update_array_iterator(iterator, |iterated, _| *iterated = None)?;
            return Ok(Value::Object(
                self.create_iter_result_object(Value::Undefined, true)?,
            ));
        }
        // 11. Set iteratorObj.[[ArrayLikeNextIndex]] to index + 1.
        self.update_array_iterator(iterator, |_, next_index| *next_index = index + 1.0)?;
        // 12. Let indexNumber be 𝔽(index).
        let index_number = Value::Number(index);
        let result = match kind {
            // 13. If kind is key, then let result be indexNumber.
            ArrayIterationKind::Key => index_number,
            // 14.a. Let elementKey be ! ToString(indexNumber). 14.b. Let elementValue be ?
            // Get(array, elementKey).
            ArrayIterationKind::Value | ArrayIterationKind::KeyValue => {
                let key = self.index_key(index)?;
                let element = self.get(array, key)?;
                match kind {
                    // 14.c. If kind is value, then let result be elementValue.
                    ArrayIterationKind::Value => element,
                    // 14.d. Else, let result be CreateArrayFromList(« indexNumber, elementValue
                    // »). No code runs while it is made, so elementValue needs no root.
                    _ => Value::Object(self.create_array_from_list(&[index_number, element])?),
                }
            }
        };
        // 15. Return CreateIteratorResultObject(result, false).
        Ok(Value::Object(
            self.create_iter_result_object(result, false)?,
        ))
    }

    /// Change the slots [[IteratedArrayLike]] and [[ArrayLikeNextIndex]] of the Array Iterator
    /// `iterator` by `change`; an object without them here is a broken invariant.
    fn update_array_iterator(
        &mut self,
        iterator: ObjectRef,
        change: impl FnOnce(&mut Option<ObjectRef>, &mut f64),
    ) -> Completion<()> {
        let changed = match &mut self.engine_object_mut(iterator)?.kind {
            ObjectKind::Ordinary(ordinary) => match &mut ordinary.data {
                OrdinaryData::ArrayIterator {
                    iterated,
                    next_index,
                    ..
                } => {
                    change(iterated, next_index);
                    true
                }
                _ => false,
            },
            _ => false,
        };
        if changed { Ok(()) } else { Err(self.fault()) }
    }

    /// `%StringIteratorPrototype%.next ( )` (`#sec-%stringiteratorprototype%.next`): what
    /// GeneratorResume of the closure of `String.prototype[%Symbol.iterator%]` does
    /// (`#sec-string.prototype-%symbol.iterator%`, step 4), over the slots that hold its state.
    /// No code runs: the String is held by the iterator, the iterator by the activation.
    fn string_iterator_next(&mut self, this: Value) -> Completion<Value> {
        // GeneratorValidate: a this value that is not a String Iterator throws a TypeError.
        let slots = match this {
            Value::Object(iterator) => match &self.engine_object(iterator)?.kind {
                ObjectKind::Ordinary(ordinary) => match ordinary.data {
                    OrdinaryData::StringIterator { iterated, position } => {
                        Some((iterator, iterated, position))
                    }
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        };
        let Some((iterator, iterated, position)) = slots else {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "%StringIteratorPrototype%.next requires that 'this' be a String Iterator",
            ));
        };
        // A completed generator answers CreateIteratorResultObject(undefined, true).
        let Some(string) = iterated else {
            return Ok(Value::Object(
                self.create_iter_result_object(Value::Undefined, true)?,
            ));
        };
        // 4.a. Let length be the length of string. 4.c. Repeat, while position < length; after
        // it, 4.d. Return NormalCompletion(unused): the generator completes and drops string.
        let length = self.string_length(string)?;
        if position >= length {
            self.update_string_iterator(iterator, None, position)?;
            return Ok(Value::Object(
                self.create_iter_result_object(Value::Undefined, true)?,
            ));
        }
        // 4.c.i. Let codePoint be CodePointAt(string, position), which reads at most two code
        // units. 4.c.ii. Let nextIndex be position + codePoint.[[CodeUnitCount]]. 4.c.iii. Let
        // resultString be the substring of string from position to nextIndex.
        let units =
            self.string_units_in(string, position, position.saturating_add(2).min(length))?;
        let Some(code_point) = code_point_at(&units, 0) else {
            return Err(self.fault());
        };
        let Some(piece) = units.get(..code_point.code_unit_count) else {
            return Err(self.fault());
        };
        let piece = self.new_string_value(piece)?;
        // 4.c.iv. Set position to nextIndex.
        self.update_string_iterator(
            iterator,
            Some(string),
            position.saturating_add(code_point.code_unit_count),
        )?;
        // 4.c.v. Perform ? GeneratorYield(CreateIteratorResultObject(resultString, false)).
        Ok(Value::Object(
            self.create_iter_result_object(Value::String(piece), false)?,
        ))
    }

    /// Set the String and the position the String Iterator `iterator` holds; an object
    /// without them here is a broken invariant.
    fn update_string_iterator(
        &mut self,
        iterator: ObjectRef,
        string: Option<StringRef>,
        next: usize,
    ) -> Completion<()> {
        let changed = match &mut self.engine_object_mut(iterator)?.kind {
            ObjectKind::Ordinary(ordinary) => match &mut ordinary.data {
                OrdinaryData::StringIterator { iterated, position } => {
                    *iterated = string;
                    *position = next;
                    true
                }
                _ => false,
            },
            _ => false,
        };
        if changed { Ok(()) } else { Err(self.fault()) }
    }
}
