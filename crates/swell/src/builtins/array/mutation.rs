//! The mutation and sort methods of `Array.prototype` (ECMA-262 23.1.3,
//! `#sec-properties-of-the-array-prototype-object`), the group of
//! `story:array-mutation-and-sort-methods`, in the order of their clauses: `copyWithin`
//! (`#sec-array.prototype.copywithin`), `fill` (`#sec-array.prototype.fill`), `pop`
//! (`#sec-array.prototype.pop`), `reverse` (`#sec-array.prototype.reverse`), `shift`
//! (`#sec-array.prototype.shift`), `sort` (`#sec-array.prototype.sort`) with
//! SortIndexedProperties (`#sec-sortindexedproperties`) and CompareArrayElements
//! (`#sec-comparearrayelements`), `splice` (`#sec-array.prototype.splice`), `toSorted`
//! (`#sec-array.prototype.tosorted`) and `unshift` (`#sec-array.prototype.unshift`).
//! ToClampedIndex (`#sec-toclampedindex`) is the one of `ops/conversion.rs`.
//!
//! Every loop over indices charges a step per index (decision 2), so a length of 2^32 - 1 or
//! 2^53 - 1 ends in `Terminated(StepBudget)`. What a method holds across a Get, a Set, a
//! getter, a setter or a comparator call (the object O, the array it makes, an element it read
//! and is about to write) is on the agent's kept stack (decision 4), released when the method
//! ends; a value read and passed to the next Set at once is held by that Set's call.
//!
//! The sort is a stable merge sort, bottom up, over a list of the values on the kept stack: the
//! values SortIndexedProperties reads and, above them, a buffer of as many, so that every value
//! is on the kept stack while a comparator or a `toString` runs. The growth of the kept stack
//! the list needs is charged to the memory budget before it is made and credited when the sort
//! ends (decision 4); a step is charged per comparison and per element moved. A merge sort
//! makes at most n log2 n comparisons whatever the comparator answers, so a comparator that
//! answers inconsistently ends the sort in some order of the values read, and one that throws
//! ends it in its throw before anything is written (`#sec-sortindexedproperties`, step 4).

use crate::builtins::object::argument;
use crate::builtins::string::units_of;
use crate::builtins::{Method, SymbolData};
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};
use crate::object::builtin_function::KeptRoom;
use crate::value::{ObjectRef, Value};

use super::MAX_SAFE_LENGTH;

/// The builtins of this group, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MutationBuiltin {
    /// `Array.prototype.copyWithin ( target, start [ , end ] )` (23.1.3.4).
    CopyWithin,
    /// `Array.prototype.fill ( value [ , start [ , end ] ] )` (23.1.3.7).
    Fill,
    /// `Array.prototype.pop ( )` (23.1.3.22).
    Pop,
    /// `Array.prototype.reverse ( )` (23.1.3.26).
    Reverse,
    /// `Array.prototype.shift ( )` (23.1.3.27).
    Shift,
    /// `Array.prototype.sort ( comparator )` (23.1.3.30).
    Sort,
    /// `Array.prototype.splice ( start, deleteCount, ...items )` (23.1.3.31).
    Splice,
    /// `Array.prototype.toSorted ( comparator )` (23.1.3.34).
    ToSorted,
    /// `Array.prototype.unshift ( ...items )` (23.1.3.37).
    Unshift,
}

/// The methods this group defines on `%Array%`, in the order of their clauses.
pub(crate) const CONSTRUCTOR_METHODS: &[Method<MutationBuiltin>] = &[];

/// The methods this group defines on `%Array.prototype%`, in the order of their clauses, each
/// with the "length" its clause gives.
pub(crate) const PROTOTYPE_METHODS: &[Method<MutationBuiltin>] = &[
    ("copyWithin", 2, MutationBuiltin::CopyWithin),
    ("fill", 1, MutationBuiltin::Fill),
    ("pop", 0, MutationBuiltin::Pop),
    ("reverse", 0, MutationBuiltin::Reverse),
    ("shift", 0, MutationBuiltin::Shift),
    ("sort", 1, MutationBuiltin::Sort),
    ("splice", 2, MutationBuiltin::Splice),
    ("toSorted", 1, MutationBuiltin::ToSorted),
    ("unshift", 1, MutationBuiltin::Unshift),
];

/// The data properties keyed by a well-known symbol this group defines on
/// `%Array.prototype%`, after its methods.
pub(crate) fn prototype_symbol_data<H: Host>() -> [SymbolData<H>; 0] {
    []
}

/// The steps of a method after ToObject of its this value, with O kept.
type Steps<H> = fn(&mut Agent<H>, ObjectRef, &[Value]) -> Completion<Value>;

/// The list of a sort on the kept stack: `count` values from `base`, and above them, while the
/// merge runs, a buffer of as many. `room` holds the room the kept stack had before and the
/// bytes its growth was charged to the memory budget.
struct SortList {
    base: usize,
    count: usize,
    room: KeptRoom,
}

/// Which indices SortIndexedProperties reads (`#sec-sortindexedproperties`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Holes {
    /// `skip-holes`: only the indices HasProperty finds.
    Skip,
    /// `read-through-holes`: every index below the length.
    ReadThrough,
}

impl<H: Host> Agent<H> {
    /// The steps of the builtins of this group; not inlined, so that the frame of
    /// `run_array_builtin` does not grow with its arms.
    #[inline(never)]
    pub(crate) fn run_array_mutation_builtin(
        &mut self,
        id: MutationBuiltin,
        _function: ObjectRef,
        this: Value,
        args: &[Value],
        _new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        match id {
            MutationBuiltin::CopyWithin => self.with_object(this, args, Self::copy_within_steps),
            MutationBuiltin::Fill => self.with_object(this, args, Self::fill_steps),
            MutationBuiltin::Pop => self.with_object(this, args, Self::pop_steps),
            MutationBuiltin::Reverse => self.with_object(this, args, Self::reverse_steps),
            MutationBuiltin::Shift => self.with_object(this, args, Self::shift_steps),
            MutationBuiltin::Sort => {
                // 1. If comparator is not undefined and IsCallable(comparator) is false, throw a
                // TypeError exception: before ToObject and before "length" is read.
                self.check_comparator(argument(args, 0), "Array.prototype.sort")?;
                self.with_object(this, args, Self::sort_steps)
            }
            MutationBuiltin::Splice => self.with_object(this, args, Self::splice_steps),
            MutationBuiltin::ToSorted => {
                // 1. The same check as `sort`'s step 1.
                self.check_comparator(argument(args, 0), "Array.prototype.toSorted")?;
                self.with_object(this, args, Self::to_sorted_steps)
            }
            MutationBuiltin::Unshift => self.with_object(this, args, Self::unshift_steps),
        }
    }

    /// "Let O be ? ToObject(this value)." O may be new and the steps run code: it is kept
    /// until `steps` end, and so is everything `steps` keep.
    fn with_object(&mut self, this: Value, args: &[Value], steps: Steps<H>) -> Completion<Value> {
        let object = self.to_object(this)?;
        let mark = self.keep(Value::Object(object));
        let answer = steps(self, object, args);
        self.release(mark);
        answer
    }

    /// "Perform ? Set(O, "length", 𝔽(length), true)."
    fn set_length(&mut self, object: ObjectRef, length: f64) -> Completion<()> {
        let key = self.name_key("length")?;
        self.set(object, key, Value::Number(length), true)?;
        Ok(())
    }

    /// The move of one element the loops of `shift`, `unshift`, `splice` and `copyWithin`
    /// share: "Let fromPresent be ? HasProperty(O, from). If fromPresent is true, then let
    /// fromValue be ? Get(O, from) and perform ? Set(O, to, fromValue, true). Else, perform ?
    /// DeletePropertyOrThrow(O, to)." The keys are made again after each step that may run
    /// code, which may collect the atom of an index above 2^32 - 2; fromValue goes to the Set
    /// at once, which holds it in its call.
    fn move_element(&mut self, object: ObjectRef, from: f64, to: f64) -> Completion<()> {
        let key = self.index_key(from)?;
        if self.has_property(object, key)? {
            let key = self.index_key(from)?;
            let value = self.get(object, key)?;
            let key = self.index_key(to)?;
            self.set(object, key, value, true)?;
        } else {
            let key = self.index_key(to)?;
            self.delete_property_or_throw(object, key)?;
        }
        Ok(())
    }

    /// Steps 2 to 12 of `copyWithin` (`#sec-array.prototype.copywithin`), with O kept.
    fn copy_within_steps(&mut self, object: ObjectRef, args: &[Value]) -> Completion<Value> {
        // 2. Let len be ? LengthOfArrayLike(O).
        let length = self.length_of_array_like(object)?;
        // 3, 4. Let to be ? ToClampedIndex(target, len); let from be ? ToClampedIndex(start,
        // len).
        let mut to = self.to_clamped_index(argument(args, 0), length)?;
        let mut from = self.to_clamped_index(argument(args, 1), length)?;
        // 5. If end is undefined, let final be len; else ? ToClampedIndex(end, len).
        let end = match argument(args, 2) {
            Value::Undefined => length,
            end => self.to_clamped_index(end, length)?,
        };
        // 6. Let count be min(final - from, len - to).
        let mut count = (end - from).min(length - to);
        // 7, 8. Backwards when the target overlaps the source from above.
        let direction = if from < to && to < from + count {
            from += count - 1.0;
            to += count - 1.0;
            -1.0
        } else {
            1.0
        };
        // 9. Repeat, while count > 0.
        while count > 0.0 {
            self.charge_steps(1)?;
            self.move_element(object, from, to)?;
            from += direction;
            to += direction;
            count -= 1.0;
        }
        // 10. Return O.
        Ok(Value::Object(object))
    }

    /// Steps 2 to 7 of `fill` (`#sec-array.prototype.fill`), with O kept. The value is held by
    /// this builtin's activation.
    fn fill_steps(&mut self, object: ObjectRef, args: &[Value]) -> Completion<Value> {
        // 2. Let len be ? LengthOfArrayLike(O).
        let length = self.length_of_array_like(object)?;
        // 3. Let k be ? ToClampedIndex(start, len).
        let mut index = self.to_clamped_index(argument(args, 1), length)?;
        // 4. If end is undefined, let final be len; else ? ToClampedIndex(end, len).
        let end = match argument(args, 2) {
            Value::Undefined => length,
            end => self.to_clamped_index(end, length)?,
        };
        // 5. Repeat, while k < final: ? Set(O, ! ToString(𝔽(k)), value, true).
        let value = argument(args, 0);
        while index < end {
            self.charge_steps(1)?;
            let key = self.index_key(index)?;
            self.set(object, key, value, true)?;
            index += 1.0;
        }
        // 6. Return O.
        Ok(Value::Object(object))
    }

    /// Steps 2 to 9 of `pop` (`#sec-array.prototype.pop`), with O kept; the element is kept
    /// across the Delete and the Set of "length", which may run a setter.
    fn pop_steps(&mut self, object: ObjectRef, _args: &[Value]) -> Completion<Value> {
        // 2. Let len be ? LengthOfArrayLike(O).
        let length = self.length_of_array_like(object)?;
        // 3. If len = 0: ? Set(O, "length", +0𝔽, true); return undefined.
        if length == 0.0 {
            self.set_length(object, 0.0)?;
            return Ok(Value::Undefined);
        }
        // 5, 6. Let newLen be 𝔽(len - 1); let index be ! ToString(newLen).
        let new_length = length - 1.0;
        // 7. Let element be ? Get(O, index).
        let key = self.index_key(new_length)?;
        let element = self.get(object, key)?;
        self.keep(element);
        // 8. Perform ? DeletePropertyOrThrow(O, index).
        let key = self.index_key(new_length)?;
        self.delete_property_or_throw(object, key)?;
        // 9. Perform ? Set(O, "length", newLen, true).
        self.set_length(object, new_length)?;
        // 10. Return element.
        Ok(element)
    }

    /// Steps 2 to 7 of `reverse` (`#sec-array.prototype.reverse`), with O kept.
    fn reverse_steps(&mut self, object: ObjectRef, _args: &[Value]) -> Completion<Value> {
        // 2. Let len be ? LengthOfArrayLike(O).
        let length = self.length_of_array_like(object)?;
        // 3, 4. Let middle be floor(len / 2); let lower be 0.
        let middle = (length / 2.0).floor();
        let mut lower = 0.0;
        // 5. Repeat, while lower ≠ middle.
        while lower != middle {
            self.charge_steps(1)?;
            let mark = self.kept.len();
            let swapped = self.reverse_pair(object, lower, length - lower - 1.0);
            self.release(mark);
            swapped?;
            lower += 1.0;
        }
        // 6. Return O.
        Ok(Value::Object(object))
    }

    /// Steps 5.a to 5.l of `reverse` for one pair; the values read are kept until the caller
    /// releases them.
    fn reverse_pair(&mut self, object: ObjectRef, lower: f64, upper: f64) -> Completion<()> {
        // d, e. Let lowerExists be ? HasProperty(O, lowerP); if true, lowerValue = ? Get.
        let key = self.index_key(lower)?;
        let lower_value = if self.has_property(object, key)? {
            let key = self.index_key(lower)?;
            let value = self.get(object, key)?;
            self.keep(value);
            Some(value)
        } else {
            None
        };
        // f, g. Let upperExists be ? HasProperty(O, upperP); if true, upperValue = ? Get.
        let key = self.index_key(upper)?;
        let upper_value = if self.has_property(object, key)? {
            let key = self.index_key(upper)?;
            let value = self.get(object, key)?;
            self.keep(value);
            Some(value)
        } else {
            None
        };
        match (lower_value, upper_value) {
            // h. Both exist: ? Set(O, lowerP, upperValue, true); ? Set(O, upperP, lowerValue,
            // true).
            (Some(lower_value), Some(upper_value)) => {
                let key = self.index_key(lower)?;
                self.set(object, key, upper_value, true)?;
                let key = self.index_key(upper)?;
                self.set(object, key, lower_value, true)?;
            }
            // i. Only upper: ? Set(O, lowerP, upperValue, true); ? DeletePropertyOrThrow(O,
            // upperP).
            (None, Some(upper_value)) => {
                let key = self.index_key(lower)?;
                self.set(object, key, upper_value, true)?;
                let key = self.index_key(upper)?;
                self.delete_property_or_throw(object, key)?;
            }
            // j. Only lower: ? DeletePropertyOrThrow(O, lowerP); ? Set(O, upperP, lowerValue,
            // true).
            (Some(lower_value), None) => {
                let key = self.index_key(lower)?;
                self.delete_property_or_throw(object, key)?;
                let key = self.index_key(upper)?;
                self.set(object, key, lower_value, true)?;
            }
            // k. Neither: no action is required.
            (None, None) => {}
        }
        Ok(())
    }

    /// Steps 2 to 9 of `shift` (`#sec-array.prototype.shift`), with O kept; the first element
    /// is kept across the loop.
    fn shift_steps(&mut self, object: ObjectRef, _args: &[Value]) -> Completion<Value> {
        // 2. Let len be ? LengthOfArrayLike(O).
        let length = self.length_of_array_like(object)?;
        // 3. If len = 0: ? Set(O, "length", +0𝔽, true); return undefined.
        if length == 0.0 {
            self.set_length(object, 0.0)?;
            return Ok(Value::Undefined);
        }
        // 4. Let first be ? Get(O, "0").
        let key = self.index_key(0.0)?;
        let first = self.get(object, key)?;
        self.keep(first);
        // 5, 6. Let k be 1; repeat, while k < len: move k to k - 1.
        let mut index = 1.0;
        while index < length {
            self.charge_steps(1)?;
            self.move_element(object, index, index - 1.0)?;
            index += 1.0;
        }
        // 7. Perform ? DeletePropertyOrThrow(O, ! ToString(𝔽(len - 1))).
        let key = self.index_key(length - 1.0)?;
        self.delete_property_or_throw(object, key)?;
        // 8. Perform ? Set(O, "length", 𝔽(len - 1), true).
        self.set_length(object, length - 1.0)?;
        // 9. Return first.
        Ok(first)
    }

    /// Steps 1 to 21 of `splice` (`#sec-array.prototype.splice`), with O kept; the array of the
    /// deleted elements is kept from its creation. The items are held by this builtin's
    /// activation.
    fn splice_steps(&mut self, object: ObjectRef, args: &[Value]) -> Completion<Value> {
        // 2. Let len be ? LengthOfArrayLike(O).
        let length = self.length_of_array_like(object)?;
        // 3. Let actualStart be ? ToClampedIndex(start, len).
        let start = self.to_clamped_index(argument(args, 0), length)?;
        // 4. Let itemCount be the number of elements in items.
        let items = args.get(2..).unwrap_or(&[]);
        let item_count = items.len() as f64;
        // 5 to 7. The delete count: 0 without a start, the rest without a delete count, and
        // otherwise ToIntegerOrInfinity(deleteCount) clamped between 0 and len - actualStart.
        let delete_count = match args.len() {
            0 => 0.0,
            1 => length - start,
            _ => {
                let count = self.to_integer_or_infinity(argument(args, 1))?;
                count.max(0.0).min(length - start)
            }
        };
        // 8. If len + itemCount - actualDeleteCount > 2^53 - 1, throw a TypeError exception.
        if length + item_count - delete_count > MAX_SAFE_LENGTH {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "Array.prototype.splice would make a length above 2^53 - 1",
            ));
        }
        // 9. Let A be ? ArraySpeciesCreate(O, actualDeleteCount).
        let deleted = self.array_species_create(object, delete_count)?;
        self.keep(Value::Object(deleted));
        // 10, 11. The deleted elements, holes skipped.
        let mut index = 0.0;
        while index < delete_count {
            self.charge_steps(1)?;
            let key = self.index_key(start + index)?;
            if self.has_property(object, key)? {
                let key = self.index_key(start + index)?;
                let value = self.get(object, key)?;
                let key = self.index_key(index)?;
                self.create_data_property_or_throw(deleted, key, value)?;
            }
            index += 1.0;
        }
        // 12. Perform ? Set(A, "length", 𝔽(actualDeleteCount), true).
        self.set_length(deleted, delete_count)?;
        if item_count < delete_count {
            // 13.a, 13.b. Move the elements after the deleted ones down.
            let mut index = start;
            while index < length - delete_count {
                self.charge_steps(1)?;
                self.move_element(object, index + delete_count, index + item_count)?;
                index += 1.0;
            }
            // 13.c, 13.d. Delete the indices at the end, the highest first.
            let mut index = length;
            while index > length - delete_count + item_count {
                self.charge_steps(1)?;
                let key = self.index_key(index - 1.0)?;
                self.delete_property_or_throw(object, key)?;
                index -= 1.0;
            }
        } else if item_count > delete_count {
            // 14. Move the elements after the deleted ones up, the highest first.
            let mut index = length - delete_count;
            while index > start {
                self.charge_steps(1)?;
                self.move_element(object, index + delete_count - 1.0, index + item_count - 1.0)?;
                index -= 1.0;
            }
        }
        // 15, 16. Write the items from actualStart.
        let mut index = start;
        for &item in items {
            self.charge_steps(1)?;
            let key = self.index_key(index)?;
            self.set(object, key, item, true)?;
            index += 1.0;
        }
        // 17. Perform ? Set(O, "length", 𝔽(len - actualDeleteCount + itemCount), true).
        self.set_length(object, length - delete_count + item_count)?;
        // 18. Return A.
        Ok(Value::Object(deleted))
    }

    /// Steps 2 to 6 of `unshift` (`#sec-array.prototype.unshift`), with O kept. The items are
    /// held by this builtin's activation.
    fn unshift_steps(&mut self, object: ObjectRef, items: &[Value]) -> Completion<Value> {
        // 2. Let len be ? LengthOfArrayLike(O).
        let length = self.length_of_array_like(object)?;
        // 3. Let argCount be the number of elements in items.
        let count = items.len() as f64;
        // 4. If argCount > 0, then
        if !items.is_empty() {
            // a. If len + argCount > 2^53 - 1, throw a TypeError exception.
            if length + count > MAX_SAFE_LENGTH {
                return Err(self.throw_error(
                    ErrorKind::TypeError,
                    "Array.prototype.unshift would make a length above 2^53 - 1",
                ));
            }
            // b, c. Let k be len; repeat, while k > 0: move k - 1 to k + argCount - 1.
            let mut index = length;
            while index > 0.0 {
                self.charge_steps(1)?;
                self.move_element(object, index - 1.0, index + count - 1.0)?;
                index -= 1.0;
            }
            // d, e. Write the items from index 0.
            let mut index = 0.0;
            for &item in items {
                self.charge_steps(1)?;
                let key = self.index_key(index)?;
                self.set(object, key, item, true)?;
                index += 1.0;
            }
        }
        // 5. Perform ? Set(O, "length", 𝔽(len + argCount), true).
        self.set_length(object, length + count)?;
        // 6. Return 𝔽(len + argCount).
        Ok(Value::Number(length + count))
    }

    /// Step 1 of `sort` and `toSorted`: "If comparator is not undefined and
    /// IsCallable(comparator) is false, throw a TypeError exception."
    fn check_comparator(&mut self, comparator: Value, method: &str) -> Completion<()> {
        if comparator != Value::Undefined && !self.is_callable(comparator)? {
            let message = format!("{method}: the comparator is not a function");
            return Err(self.throw_error(ErrorKind::TypeError, &message));
        }
        Ok(())
    }

    /// Steps 3 to 11 of `sort` (`#sec-array.prototype.sort`), with O kept. The comparator is
    /// held by this builtin's activation. The reads and the writes are functions of their own,
    /// so that while a comparator runs only this frame, the merge's and CompareArrayElements'
    /// lie between the builtin's entry and the call (`STACK_BYTES_PER_REENTRY`).
    fn sort_steps(&mut self, object: ObjectRef, args: &[Value]) -> Completion<Value> {
        // 3. Let len be ? LengthOfArrayLike(O).
        let length = self.length_of_array_like(object)?;
        // 5. Let sortedList be ? SortIndexedProperties(O, len, SortCompare, skip-holes).
        let mut list = self.sort_list();
        let mut done = self.read_sort_items(object, length, Holes::Skip, &mut list);
        if done.is_ok() {
            done = self.merge_sort(&mut list, argument(args, 0));
        }
        // 6 to 10. Write sortedList from index 0, then delete as many indices as there were
        // holes.
        if done.is_ok() {
            done = self.write_sorted(object, length, &list);
        }
        // The end, whatever its completion: the list and its buffer are released, the kept
        // stack is given back the room it had and its growth is credited.
        self.end_kept_room(list.base, &list.room);
        done?;
        // 11. Return O.
        Ok(Value::Object(object))
    }

    /// Steps 6 to 10 of `sort`: each element of the sorted list is written from index 0, then
    /// the indices from itemCount to len are deleted.
    fn write_sorted(&mut self, object: ObjectRef, length: f64, list: &SortList) -> Completion<()> {
        // 6 to 8. ? Set(O, ! ToString(𝔽(j)), sortedList[j], true) for each j.
        let mut index = 0.0;
        for position in 0..list.count {
            self.charge_steps(1)?;
            let value = self.kept_at(list.base.saturating_add(position))?;
            let key = self.index_key(index)?;
            self.set(object, key, value, true)?;
            index += 1.0;
        }
        // 9, 10. ? DeletePropertyOrThrow(O, ! ToString(𝔽(j))) while j < len.
        while index < length {
            self.charge_steps(1)?;
            let key = self.index_key(index)?;
            self.delete_property_or_throw(object, key)?;
            index += 1.0;
        }
        Ok(())
    }

    /// Steps 3 to 9 of `toSorted` (`#sec-array.prototype.tosorted`), with O kept; the new
    /// Array is kept from its creation. The comparator is held by this builtin's activation.
    #[expect(
        clippy::wrong_self_convention,
        reason = "the name is ECMA-262's `toSorted`; the steps call into the agent, so they need `&mut self`"
    )]
    fn to_sorted_steps(&mut self, object: ObjectRef, args: &[Value]) -> Completion<Value> {
        // 3. Let len be ? LengthOfArrayLike(O).
        let length = self.length_of_array_like(object)?;
        // 4. Let A be ? ArrayCreate(len).
        let array = self.array_create(length, None)?;
        self.keep(Value::Object(array));
        // 6. Let sortedList be ? SortIndexedProperties(O, len, SortCompare,
        // read-through-holes).
        let mut list = self.sort_list();
        let mut done = self.read_sort_items(object, length, Holes::ReadThrough, &mut list);
        if done.is_ok() {
            done = self.merge_sort(&mut list, argument(args, 0));
        }
        // 7, 8. Define the elements of A.
        if done.is_ok() {
            done = self.fill_sorted(array, &list);
        }
        // The end, whatever its completion: the list and its buffer are released, the kept
        // stack is given back the room it had and its growth is credited.
        self.end_kept_room(list.base, &list.room);
        done?;
        // 9. Return A.
        Ok(Value::Object(array))
    }

    /// Steps 7 and 8 of `toSorted`: "Perform ! CreateDataPropertyOrThrow(A, ! ToString(𝔽(j)),
    /// sortedList[j])" for each j below len.
    fn fill_sorted(&mut self, array: ObjectRef, list: &SortList) -> Completion<()> {
        let mut index = 0.0;
        for position in 0..list.count {
            self.charge_steps(1)?;
            let value = self.kept_at(list.base.saturating_add(position))?;
            let key = self.index_key(index)?;
            self.create_data_property_or_throw(array, key, value)?;
            index += 1.0;
        }
        Ok(())
    }

    /// A new, empty sort list at the top of the kept stack.
    fn sort_list(&self) -> SortList {
        SortList {
            base: self.kept.len(),
            count: 0,
            room: self.kept_room(),
        }
    }

    /// The value at `position` of the kept stack; a position past its end is an engine bug.
    fn kept_at(&mut self, position: usize) -> Completion<Value> {
        match self.kept.get(position) {
            Some(&value) => Ok(value),
            None => Err(self.fault()),
        }
    }

    /// Write `value` at `position` of the kept stack.
    fn kept_put(&mut self, position: usize, value: Value) -> Completion<()> {
        if let Some(slot) = self.kept.get_mut(position) {
            *slot = value;
            return Ok(());
        }
        Err(self.fault())
    }

    /// Steps 1 to 3 of SortIndexedProperties ( obj, len, SortCompare, holes )
    /// (`#sec-sortindexedproperties`): the values read, a step per index, are appended to
    /// `list`.
    fn read_sort_items(
        &mut self,
        object: ObjectRef,
        length: f64,
        holes: Holes,
        list: &mut SortList,
    ) -> Completion<()> {
        // For each k below len: with skip-holes only where HasProperty finds it, ? Get.
        let mut index = 0.0;
        while index < length {
            self.charge_steps(1)?;
            let read = match holes {
                Holes::Skip => {
                    let key = self.index_key(index)?;
                    self.has_property(object, key)?
                }
                Holes::ReadThrough => true,
            };
            if read {
                let key = self.index_key(index)?;
                let value = self.get(object, key)?;
                // Nothing runs between the Get and the push. Every call between keeps and
                // releases above the list, so the list stays one run of the kept stack.
                self.reserve_kept(&mut list.room, 1)?;
                self.kept.push(value);
                list.count = list.count.saturating_add(1);
            }
            index += 1.0;
        }
        Ok(())
    }

    /// The buffer of the merge: a copy of the list above it, charged as the kept stack grows.
    fn copy_sort_buffer(&mut self, list: &mut SortList) -> Completion<()> {
        self.reserve_kept(&mut list.room, list.count)?;
        for position in 0..list.count {
            let value = self.kept_at(list.base.saturating_add(position))?;
            self.kept.push(value);
        }
        Ok(())
    }

    /// The sorted values from `source` copied down to the list, a step per element moved.
    fn copy_sorted_down(&mut self, list: &SortList, source: usize) -> Completion<()> {
        for position in 0..list.count {
            self.charge_steps(1)?;
            let value = self.kept_at(source.saturating_add(position))?;
            self.kept_put(list.base.saturating_add(position), value)?;
        }
        Ok(())
    }

    /// Step 4 of SortIndexedProperties: "Sort items using an implementation-defined sequence of
    /// calls to SortCompare"; an abrupt completion stops it. A stable merge sort, bottom up, of
    /// the `count` values from `base` through a buffer of as many above them: each pass merges
    /// runs of `width` from one half into the other, the left value first unless SortCompare
    /// answers that it is greater, so equal values keep their order. The half merged from holds
    /// every value at every moment, and both halves are on the kept stack. A step per
    /// comparison and per element moved.
    fn merge_sort(&mut self, list: &mut SortList, comparator: Value) -> Completion<()> {
        let count = list.count;
        if count < 2 {
            return Ok(());
        }
        self.copy_sort_buffer(list)?;
        let mut source = list.base;
        let mut target = list.base.saturating_add(count);
        let mut width: usize = 1;
        while width < count {
            let mut low = 0;
            while low < count {
                let middle = low.saturating_add(width).min(count);
                let high = middle.saturating_add(width).min(count);
                let (mut left, left_end) = (source + low, source + middle);
                let (mut right, right_end) = (source + middle, source + high);
                let mut out = target + low;
                while left < left_end || right < right_end {
                    self.charge_steps(1)?;
                    let take_right = if left == left_end {
                        true
                    } else if right == right_end {
                        false
                    } else {
                        self.charge_steps(1)?;
                        let x = self.kept_at(left)?;
                        let y = self.kept_at(right)?;
                        self.compare_array_elements(x, y, comparator)? > 0.0
                    };
                    let from = if take_right { &mut right } else { &mut left };
                    let value = self.kept_at(*from)?;
                    *from += 1;
                    self.kept_put(out, value)?;
                    out += 1;
                }
                low = high;
            }
            std::mem::swap(&mut source, &mut target);
            width = width.saturating_mul(2);
        }
        // After an odd number of passes the sorted values are in the buffer.
        if source != list.base {
            self.copy_sorted_down(list, source)?;
        }
        Ok(())
    }

    /// CompareArrayElements ( x, y, comparator ) (`#sec-comparearrayelements`). x and y are on
    /// the kept stack, and the comparator's call holds them too.
    fn compare_array_elements(&mut self, x: Value, y: Value, comparator: Value) -> Completion<f64> {
        // 1 to 3. undefined sorts after every other value.
        match (x, y) {
            (Value::Undefined, Value::Undefined) => Ok(0.0),
            (Value::Undefined, _) => Ok(1.0),
            (_, Value::Undefined) => Ok(-1.0),
            // 5 to 11. Without a comparator, by ToString.
            _ if comparator == Value::Undefined => self.compare_as_strings(x, y),
            // 4. ? ToNumber(? Call(comparator, undefined, « x, y »)); NaN is +0𝔽.
            _ => {
                let result = self.call_value(comparator, Value::Undefined, &[x, y])?;
                let result = self.to_number(result)?;
                Ok(if result.is_nan() { 0.0 } else { result })
            }
        }
    }

    /// Steps 5 to 11 of CompareArrayElements: ? ToString(x) and ? ToString(y) compared by their
    /// code units (IsLessThan, `#sec-islessthan`, step 3), a step per code unit compared, as
    /// IsLessThan charges.
    fn compare_as_strings(&mut self, x: Value, y: Value) -> Completion<f64> {
        let x_units = self.sort_string_units(x)?;
        let y_units = self.sort_string_units(y)?;
        let compared = u64::try_from(x_units.len().min(y_units.len())).unwrap_or(u64::MAX);
        self.charge_steps(compared)?;
        Ok(match x_units.cmp(&y_units) {
            std::cmp::Ordering::Less => -1.0,
            std::cmp::Ordering::Greater => 1.0,
            std::cmp::Ordering::Equal => 0.0,
        })
    }

    /// The code units of ToString(value) (`#sec-tostring`) for CompareArrayElements: those of a
    /// primitive are made without a String on the heap, which a sort without a comparator
    /// would otherwise allocate twice per comparison; an object goes through ToString.
    fn sort_string_units(&mut self, value: Value) -> Completion<Vec<u16>> {
        match value {
            Value::String(string) => self.string_units(string),
            Value::Number(number) => Ok(units_of(&swell_num::to_string::number_to_string(number))),
            Value::Boolean(true) => Ok(units_of("true")),
            Value::Boolean(false) => Ok(units_of("false")),
            Value::Null => Ok(units_of("null")),
            value => {
                let string = self.to_string(value)?;
                self.string_units(string)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::host::hooks::test_host::{NoHooks, agent_with_realm, enter};
    use crate::host::outcome::{Abrupt, Completion};
    use crate::host::{Cx, ErrorKind, HostCall};
    use crate::object::ordinary::OrdinaryData;
    use crate::object::property::Key;
    use crate::ops::objects::IntegrityLevel;
    use crate::realm::intrinsics::Intrinsic;
    use crate::value::{ObjectRef, Value};

    fn number(n: f64) -> Value {
        Value::Number(n)
    }

    /// The elements below 3 and the "length" of `array`.
    fn elements_and_length(
        agent: &mut crate::host::agent::Agent<NoHooks>,
        array: ObjectRef,
    ) -> Vec<Value> {
        let mut seen: Vec<Value> = (0..3)
            .map(|i| agent.get(array, Key::Index(i)).unwrap())
            .collect();
        let length = agent.name_key("length").unwrap();
        seen.push(agent.get(array, length).unwrap());
        seen
    }

    /// `#sec-setintegritylevel` with `frozen` makes every element and the "length" of an Array
    /// neither writable nor configurable and the Array not extensible: each of the eight
    /// methods of this group that write (`copyWithin`, `fill`, `pop`, `reverse`, `shift`,
    /// `sort`, `splice`, `unshift`) throws a TypeError of the realm on the Array `[3, 1, 2]`
    /// frozen so, and every element and the length are as before; `toSorted` answers a new
    /// Array, sorted.
    #[test]
    fn a_frozen_receiver_throws_and_keeps_its_elements() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let prototype = agent.intrinsic(handle, Intrinsic::ArrayPrototype).unwrap();
        let type_error = agent
            .intrinsic(handle, Intrinsic::ErrorPrototype(ErrorKind::TypeError))
            .unwrap();
        let before = vec![number(3.0), number(1.0), number(2.0), number(3.0)];
        let make = |agent: &mut crate::host::agent::Agent<NoHooks>| {
            let array = agent.array_create(0.0, None).unwrap();
            agent.keep(Value::Object(array));
            for (i, value) in [3.0, 1.0, 2.0].into_iter().enumerate() {
                let i = u32::try_from(i).unwrap();
                agent
                    .create_data_property_or_throw(array, Key::Index(i), number(value))
                    .unwrap();
            }
            assert_eq!(
                agent.set_integrity_level(array, IntegrityLevel::Frozen),
                Ok(true)
            );
            array
        };
        let calls: [(&str, &[Value]); 8] = [
            ("copyWithin", &[number(0.0), number(1.0)]),
            ("fill", &[number(9.0)]),
            ("pop", &[]),
            ("reverse", &[]),
            ("shift", &[]),
            ("sort", &[]),
            ("splice", &[number(0.0), number(1.0)]),
            ("unshift", &[number(9.0)]),
        ];
        for (name, args) in calls {
            let array = make(&mut agent);
            let key = agent.name_key(name).unwrap();
            let method = agent.get(prototype, key).unwrap();
            assert_eq!(agent.is_callable(method), Ok(true), "{name}");
            let answer = agent.call_value(method, Value::Object(array), args);
            let Err(Abrupt::Throw(Value::Object(error))) = answer else {
                panic!("{name}: {answer:?}");
            };
            assert_eq!(
                agent.get_prototype_of(error),
                Ok(Some(type_error)),
                "{name}"
            );
            assert_eq!(elements_and_length(&mut agent, array), before, "{name}");
        }
        let array = make(&mut agent);
        let key = agent.name_key("toSorted").unwrap();
        let method = agent.get(prototype, key).unwrap();
        let Ok(Value::Object(sorted)) = agent.call_value(method, Value::Object(array), &[]) else {
            panic!("toSorted answered no object");
        };
        assert_ne!(sorted, array);
        assert_eq!(agent.is_array(Value::Object(sorted)), Ok(true));
        assert_eq!(
            elements_and_length(&mut agent, sorted),
            vec![number(1.0), number(2.0), number(3.0), number(3.0)]
        );
        assert_eq!(elements_and_length(&mut agent, array), before);
    }

    /// A species constructor that records the length it is given as the callee's "seen"
    /// property, and answers a new ordinary object.
    fn species(cx: &mut Cx<'_, NoHooks>, call: HostCall<'_>) -> Completion<Value> {
        let agent = &mut *cx.agent;
        let key = agent.name_key("seen")?;
        let length = call.args.first().copied().unwrap_or(Value::Undefined);
        agent.create_data_property(call.callee, key, length)?;
        let prototype = agent.current_intrinsic(Intrinsic::ObjectPrototype)?;
        let made = agent.ordinary_object_create(Some(prototype), OrdinaryData::Plain)?;
        Ok(Value::Object(made))
    }

    /// `#sec-array.prototype.splice`, step 10, with ArraySpeciesCreate
    /// (`#sec-arrayspeciescreate`, step 5, the part that needs @@species, which the host
    /// interface does not hand out): an Array whose constructor has an own @@species
    /// constructor gets the array of its deleted elements from that constructor, called with
    /// the delete count; `splice` defines the deleted elements on it, holes skipped, and sets its
    /// "length"; a @@species of null makes an Array; one that is not a constructor throws a
    /// TypeError before an element is moved.
    #[test]
    fn splice_uses_species() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let array_prototype = agent.intrinsic(handle, Intrinsic::ArrayPrototype).unwrap();
        let species_key = Key::Symbol(agent.well_known().unwrap().species);
        let constructor = agent
            .create_host_function(handle, "Species", 0, species, true, None)
            .unwrap();
        let owner = agent
            .create_host_function(handle, "Owner", 0, species, true, None)
            .unwrap();
        let plain = agent
            .create_host_function(handle, "plain", 0, species, false, None)
            .unwrap();
        for object in [constructor, owner, plain] {
            agent.keep(Value::Object(object));
        }
        let constructor_key = agent.name_key("constructor").unwrap();
        agent
            .create_data_property(owner, species_key, Value::Object(constructor))
            .unwrap();
        let splice_key = agent.name_key("splice").unwrap();
        let splice = agent.get(array_prototype, splice_key).unwrap();
        // [ "a", hole, 2, 3 ] with its own "constructor" Owner.
        let source = |agent: &mut crate::host::agent::Agent<NoHooks>| {
            let source = agent.array_create(0.0, None).unwrap();
            agent.keep(Value::Object(source));
            let a = Value::String(agent.new_string_str("a").unwrap());
            agent
                .create_data_property(source, Key::Index(0), a)
                .unwrap();
            for i in [2, 3] {
                agent
                    .create_data_property(source, Key::Index(i), number(f64::from(i)))
                    .unwrap();
            }
            agent
                .create_data_property(source, constructor_key, Value::Object(owner))
                .unwrap();
            source
        };
        let array = source(&mut agent);
        let Ok(Value::Object(deleted)) =
            agent.call_value(splice, Value::Object(array), &[number(0.0), number(3.0)])
        else {
            panic!("splice answered no object");
        };
        let seen_key = agent.name_key("seen").unwrap();
        assert_eq!(agent.get(constructor, seen_key), Ok(number(3.0)));
        assert_eq!(agent.is_array(Value::Object(deleted)), Ok(false));
        let a = agent.get(deleted, Key::Index(0)).unwrap();
        let Value::String(a) = a else {
            panic!("{a:?}");
        };
        assert_eq!(agent.string_units(a).unwrap(), vec![u16::from(b'a')]);
        assert_eq!(agent.has_property(deleted, Key::Index(1)), Ok(false));
        assert_eq!(agent.get(deleted, Key::Index(2)), Ok(number(2.0)));
        let length_key = agent.name_key("length").unwrap();
        assert_eq!(agent.get(deleted, length_key), Ok(number(3.0)));
        assert_eq!(agent.get(array, length_key), Ok(number(1.0)));
        assert_eq!(agent.get(array, Key::Index(0)), Ok(number(3.0)));

        // A @@species of null makes an Array; one that is not a constructor throws.
        let species_property =
            |value| crate::object::property::Descriptor::data(value, true, true, true);
        agent
            .define_own_property(owner, species_key, species_property(Value::Null))
            .unwrap();
        let array = source(&mut agent);
        let Ok(Value::Object(deleted)) =
            agent.call_value(splice, Value::Object(array), &[number(1.0)])
        else {
            panic!("splice answered no object");
        };
        assert_eq!(agent.is_array(Value::Object(deleted)), Ok(true));
        assert_eq!(agent.get(deleted, length_key), Ok(number(3.0)));
        agent
            .define_own_property(owner, species_key, species_property(Value::Object(plain)))
            .unwrap();
        let array = source(&mut agent);
        assert!(matches!(
            agent.call_value(splice, Value::Object(array), &[number(0.0), number(1.0)]),
            Err(Abrupt::Throw(_))
        ));
        assert_eq!(agent.get(array, length_key), Ok(number(4.0)));
        assert_eq!(agent.get(array, Key::Index(2)), Ok(number(2.0)));
    }
}
