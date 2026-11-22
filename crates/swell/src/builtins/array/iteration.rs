//! The iteration methods of `Array.prototype` (ECMA-262 23.1.3,
//! `#sec-properties-of-the-array-prototype-object`), the group of
//! `story:array-iteration-methods`: `every` (`#sec-array.prototype.every`), `filter`
//! (`#sec-array.prototype.filter`), `find` (`#sec-array.prototype.find`), `findIndex`
//! (`#sec-array.prototype.findindex`), `findLast` (`#sec-array.prototype.findlast`) and
//! `findLastIndex` (`#sec-array.prototype.findlastindex`) with FindViaPredicate
//! (`#sec-findviapredicate`), `flat` (`#sec-array.prototype.flat`) and `flatMap`
//! (`#sec-array.prototype.flatmap`) with FlattenIntoArray (`#sec-flattenintoarray`), `forEach`
//! (`#sec-array.prototype.foreach`), `reduce` (`#sec-array.prototype.reduce`), `reduceRight`
//! (`#sec-array.prototype.reduceright`) and `some` (`#sec-array.prototype.some`).
//!
//! Every walk charges a step per index it visits (decision 2), holes included, so a length of
//! 2^32 - 1 or 2^53 - 1 without elements ends in `Terminated(StepBudget)`. What a walk holds
//! across a call into code is on the agent's kept stack, which the root site of
//! `object/builtin_function.rs` reports (decision 4): O from its ToObject on, the array a method
//! fills, a value read before a callback and used after it, the accumulator of `reduce` and
//! `reduceRight`, and every source FlattenIntoArray has open. A key is made again after each
//! call: a call may collect the atom of an index above 2^32 - 2.
//!
//! FlattenIntoArray recurses once per nested Array in the clause; here it is a loop over an
//! explicit stack with one entry per source being flattened, each charged to the memory budget
//! while it is open ([`OPEN_BYTES`]), so an Array that holds itself, flattened with depth
//! `Infinity`, ends in the step or the memory budget and never in a Rust stack overflow.

use super::MAX_SAFE_LENGTH;
use crate::builtins::object::argument;
use crate::builtins::{Method, SymbolData};
use crate::host::agent::Agent;
use crate::host::outcome::{Completion, Termination};
use crate::host::{ErrorKind, Host};
use crate::value::{ObjectRef, Value};

/// The builtins of this group, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IterationBuiltin {
    /// `Array.prototype.every ( callback [ , thisArg ] )` (23.1.3.6).
    Every,
    /// `Array.prototype.filter ( callback [ , thisArg ] )` (23.1.3.8).
    Filter,
    /// `Array.prototype.find ( predicate [ , thisArg ] )` (23.1.3.9).
    Find,
    /// `Array.prototype.findIndex ( predicate [ , thisArg ] )` (23.1.3.10).
    FindIndex,
    /// `Array.prototype.findLast ( predicate [ , thisArg ] )` (23.1.3.11).
    FindLast,
    /// `Array.prototype.findLastIndex ( predicate [ , thisArg ] )` (23.1.3.12).
    FindLastIndex,
    /// `Array.prototype.flat ( [ depth ] )` (23.1.3.13).
    Flat,
    /// `Array.prototype.flatMap ( mapperFunc [ , thisArg ] )` (23.1.3.14).
    FlatMap,
    /// `Array.prototype.forEach ( callback [ , thisArg ] )` (23.1.3.15).
    ForEach,
    /// `Array.prototype.reduce ( callback [ , initialValue ] )` (23.1.3.24).
    Reduce,
    /// `Array.prototype.reduceRight ( callback [ , initialValue ] )` (23.1.3.25).
    ReduceRight,
    /// `Array.prototype.some ( callback [ , thisArg ] )` (23.1.3.29).
    Some,
}

/// The methods this group defines on `%Array%`, in the order of their clauses.
pub(crate) const CONSTRUCTOR_METHODS: &[Method<IterationBuiltin>] = &[];

/// The methods this group defines on `%Array.prototype%`, in the order of their clauses.
pub(crate) const PROTOTYPE_METHODS: &[Method<IterationBuiltin>] = &[
    ("every", 1, IterationBuiltin::Every),
    ("filter", 1, IterationBuiltin::Filter),
    ("find", 1, IterationBuiltin::Find),
    ("findIndex", 1, IterationBuiltin::FindIndex),
    ("findLast", 1, IterationBuiltin::FindLast),
    ("findLastIndex", 1, IterationBuiltin::FindLastIndex),
    ("flat", 0, IterationBuiltin::Flat),
    ("flatMap", 1, IterationBuiltin::FlatMap),
    ("forEach", 1, IterationBuiltin::ForEach),
    ("reduce", 1, IterationBuiltin::Reduce),
    ("reduceRight", 1, IterationBuiltin::ReduceRight),
    ("some", 1, IterationBuiltin::Some),
];

/// The data properties keyed by a well-known symbol this group defines on
/// `%Array.prototype%`, after its methods.
pub(crate) fn prototype_symbol_data<H: Host>() -> [SymbolData<H>; 0] {
    []
}

/// What a walk of `forEach`, `every` or `some` answers and when it stops.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Quantifier {
    /// `forEach`: every element present, then undefined.
    ForEach,
    /// `every`: false at the first answer that ToBoolean makes false, else true.
    Every,
    /// `some`: true at the first answer that ToBoolean makes true, else false.
    Some,
}

/// One source FlattenIntoArray is walking: what one recursive call of the clause holds.
#[derive(Clone, Copy)]
struct Open {
    /// The source, kept on the kept stack at `mark` (the outermost is O, kept by the caller).
    source: ObjectRef,
    /// sourceLength.
    length: f64,
    /// sourceIndex, the next index to visit.
    index: f64,
    /// depth.
    depth: f64,
    /// The length of the kept stack before the source was kept there.
    mark: usize,
}

/// The bytes an open source of FlattenIntoArray is charged to the memory budget while it is
/// open: its entry on the explicit stack and its slot on the kept stack.
const OPEN_BYTES: u64 = (size_of::<Open>() + size_of::<Value>()) as u64;

impl<H: Host> Agent<H> {
    /// The steps of the builtins of this group; not inlined, so that the frame of
    /// `run_array_builtin` does not grow with its arms. Step 1 of each, `Let O be ?
    /// ToObject(this value)`, is here: O may be new and every method runs code, so it is kept
    /// until the method answers.
    #[inline(never)]
    pub(crate) fn run_array_iteration_builtin(
        &mut self,
        id: IterationBuiltin,
        _function: ObjectRef,
        this: Value,
        args: &[Value],
        _new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        let object = self.to_object(this)?;
        let mark = self.keep(Value::Object(object));
        let answer = match id {
            IterationBuiltin::Every => self.array_quantify(object, args, Quantifier::Every),
            IterationBuiltin::Filter => self.array_filter(object, args),
            IterationBuiltin::Find => self.array_find(object, args, true, false),
            IterationBuiltin::FindIndex => self.array_find(object, args, true, true),
            IterationBuiltin::FindLast => self.array_find(object, args, false, false),
            IterationBuiltin::FindLastIndex => self.array_find(object, args, false, true),
            IterationBuiltin::Flat => self.array_flat(object, args, false),
            IterationBuiltin::FlatMap => self.array_flat(object, args, true),
            IterationBuiltin::ForEach => self.array_quantify(object, args, Quantifier::ForEach),
            IterationBuiltin::Reduce => self.array_reduce(object, args, true),
            IterationBuiltin::ReduceRight => self.array_reduce(object, args, false),
            IterationBuiltin::Some => self.array_quantify(object, args, Quantifier::Some),
        };
        self.release(mark);
        answer
    }

    /// Throw a TypeError with `message` unless `value` is callable.
    fn require_callable(&mut self, value: Value, message: &str) -> Completion<()> {
        if !self.is_callable(value)? {
            return Err(self.throw_error(ErrorKind::TypeError, message));
        }
        Ok(())
    }

    /// Set the slot `mark` of the kept stack to `value`.
    fn set_kept(&mut self, mark: usize, value: Value) -> Completion<()> {
        match self.kept.get_mut(mark) {
            Some(slot) => {
                *slot = value;
                Ok(())
            }
            None => Err(self.fault()),
        }
    }

    /// The value in the slot `mark` of the kept stack.
    fn kept_value(&mut self, mark: usize) -> Completion<Value> {
        match self.kept.get(mark) {
            Some(&value) => Ok(value),
            None => Err(self.fault()),
        }
    }

    /// Steps 2 to the end of `forEach` (`#sec-array.prototype.foreach`), `every`
    /// (`#sec-array.prototype.every`) and `some` (`#sec-array.prototype.some`), with O kept.
    /// The callback and thisArg are held by this builtin's activation.
    fn array_quantify(
        &mut self,
        object: ObjectRef,
        args: &[Value],
        quantifier: Quantifier,
    ) -> Completion<Value> {
        let callback = argument(args, 0);
        let this_arg = argument(args, 1);
        // 2. Let len be ? LengthOfArrayLike(O).
        let length = self.length_of_array_like(object)?;
        // 3. If IsCallable(callback) is false, throw a TypeError exception.
        self.require_callable(
            callback,
            match quantifier {
                Quantifier::ForEach => "Array.prototype.forEach: the callback is not a function",
                Quantifier::Every => "Array.prototype.every: the callback is not a function",
                Quantifier::Some => "Array.prototype.some: the callback is not a function",
            },
        )?;
        // 4, 5. For each k below len.
        let mut index = 0.0;
        while index < length {
            self.charge_steps(1)?;
            // a, b. Let Pk be ! ToString(𝔽(k)); let kPresent be ? HasProperty(O, Pk).
            let key = self.index_key(index)?;
            if self.has_property(object, key)? {
                // c.i. Let kValue be ? Get(O, Pk).
                let key = self.index_key(index)?;
                let value = self.get(object, key)?;
                // c.ii. ? Call(callback, thisArg, « kValue, 𝔽(k), O »); for every and some,
                // ToBoolean of the answer decides whether the walk stops.
                let answer = self.call_value(
                    callback,
                    this_arg,
                    &[value, Value::Number(index), Value::Object(object)],
                )?;
                match quantifier {
                    Quantifier::ForEach => {}
                    Quantifier::Every => {
                        if !self.to_boolean(answer)? {
                            return Ok(Value::Boolean(false));
                        }
                    }
                    Quantifier::Some => {
                        if self.to_boolean(answer)? {
                            return Ok(Value::Boolean(true));
                        }
                    }
                }
            }
            index += 1.0;
        }
        // 6. Return undefined (forEach), true (every) or false (some).
        Ok(match quantifier {
            Quantifier::ForEach => Value::Undefined,
            Quantifier::Every => Value::Boolean(true),
            Quantifier::Some => Value::Boolean(false),
        })
    }

    /// Steps 2 to 8 of `filter` (`#sec-array.prototype.filter`), with O kept. The callback and
    /// thisArg are held by this builtin's activation; A is kept from its creation on, and each
    /// kValue across its call, since the callback may delete it from O and it is written after.
    fn array_filter(&mut self, object: ObjectRef, args: &[Value]) -> Completion<Value> {
        let callback = argument(args, 0);
        let this_arg = argument(args, 1);
        // 2. Let len be ? LengthOfArrayLike(O).
        let length = self.length_of_array_like(object)?;
        // 3. If IsCallable(callback) is false, throw a TypeError exception.
        self.require_callable(
            callback,
            "Array.prototype.filter: the callback is not a function",
        )?;
        // 4. Let A be ? ArraySpeciesCreate(O, 0).
        let array = self.array_species_create(object, 0.0)?;
        self.keep(Value::Object(array));
        // 5, 6, 7. k and to from 0; for each k below len.
        let mut index = 0.0;
        let mut to = 0.0;
        while index < length {
            self.charge_steps(1)?;
            // a, b. Let Pk be ! ToString(𝔽(k)); let kPresent be ? HasProperty(O, Pk).
            let key = self.index_key(index)?;
            if self.has_property(object, key)? {
                // c.i. Let kValue be ? Get(O, Pk).
                let key = self.index_key(index)?;
                let value = self.get(object, key)?;
                let mark = self.keep(value);
                // c.ii. Let selected be ToBoolean(? Call(callback, thisArg, « kValue, 𝔽(k), O »)).
                let answer = self.call_value(
                    callback,
                    this_arg,
                    &[value, Value::Number(index), Value::Object(object)],
                )?;
                // c.iii. If selected is true, ? CreateDataPropertyOrThrow(A, ! ToString(𝔽(to)),
                // kValue) and to + 1.
                if self.to_boolean(answer)? {
                    let key = self.index_key(to)?;
                    self.create_data_property_or_throw(array, key, value)?;
                    to += 1.0;
                }
                self.release(mark);
            }
            index += 1.0;
        }
        // 8. Return A.
        Ok(Value::Object(array))
    }

    /// Steps 2 and 3 of `find` (`#sec-array.prototype.find`), `findIndex`
    /// (`#sec-array.prototype.findindex`), `findLast` (`#sec-array.prototype.findlast`) and
    /// `findLastIndex` (`#sec-array.prototype.findlastindex`) with FindViaPredicate
    /// (`#sec-findviapredicate`), with O kept: ascending or descending, answering the index or
    /// the value. The predicate and thisArg are held by this builtin's activation, each kValue
    /// across its call.
    fn array_find(
        &mut self,
        object: ObjectRef,
        args: &[Value],
        ascending: bool,
        answer_index: bool,
    ) -> Completion<Value> {
        let predicate = argument(args, 0);
        let this_arg = argument(args, 1);
        // 2. Let len be ? LengthOfArrayLike(O).
        let length = self.length_of_array_like(object)?;
        // FindViaPredicate 1. If IsCallable(predicate) is false, throw a TypeError exception.
        self.require_callable(
            predicate,
            match (ascending, answer_index) {
                (true, false) => "Array.prototype.find: the predicate is not a function",
                (true, true) => "Array.prototype.findIndex: the predicate is not a function",
                (false, false) => "Array.prototype.findLast: the predicate is not a function",
                (false, true) => "Array.prototype.findLastIndex: the predicate is not a function",
            },
        )?;
        // FindViaPredicate 2, 3. The integers from 0 below len, ascending or descending; the
        // List is not made, k walks it.
        let (mut index, step) = if ascending {
            (0.0, 1.0)
        } else {
            (length - 1.0, -1.0)
        };
        // 4. For each integer k of indices.
        while (0.0..length).contains(&index) {
            self.charge_steps(1)?;
            // a, c. Let Pk be ! ToString(𝔽(k)); let kValue be ? Get(O, Pk).
            let key = self.index_key(index)?;
            let value = self.get(object, key)?;
            let mark = self.keep(value);
            // d. Let testResult be ? Call(predicate, thisArg, « kValue, 𝔽(k), O »).
            let answer = self.call_value(
                predicate,
                this_arg,
                &[value, Value::Number(index), Value::Object(object)],
            )?;
            // e. If ToBoolean(testResult) is true, return { [[Index]]: 𝔽(k), [[Value]]: kValue }.
            if self.to_boolean(answer)? {
                return Ok(if answer_index {
                    Value::Number(index)
                } else {
                    value
                });
            }
            self.release(mark);
            index += step;
        }
        // 5. Return { [[Index]]: -1𝔽, [[Value]]: undefined }.
        Ok(if answer_index {
            Value::Number(-1.0)
        } else {
            Value::Undefined
        })
    }

    /// Steps 2 to the end of `reduce` (`#sec-array.prototype.reduce`, `ascending`) and
    /// `reduceRight` (`#sec-array.prototype.reduceright`), with O kept. The callback is held by
    /// this builtin's activation; the accumulator is in a slot of the kept stack, since the
    /// callback's answer is held by nothing else while the next element is read.
    fn array_reduce(
        &mut self,
        object: ObjectRef,
        args: &[Value],
        ascending: bool,
    ) -> Completion<Value> {
        let callback = argument(args, 0);
        let initial = args.get(1).copied();
        // 2. Let len be ? LengthOfArrayLike(O).
        let length = self.length_of_array_like(object)?;
        // 3. If IsCallable(callback) is false, throw a TypeError exception.
        // 4. If len = 0 and initialValue is not present, throw a TypeError exception.
        let (not_callable, empty) = if ascending {
            (
                "Array.prototype.reduce: the callback is not a function",
                "Array.prototype.reduce of no element with no initial value",
            )
        } else {
            (
                "Array.prototype.reduceRight: the callback is not a function",
                "Array.prototype.reduceRight of no element with no initial value",
            )
        };
        self.require_callable(callback, not_callable)?;
        if length == 0.0 && initial.is_none() {
            return Err(self.throw_error(ErrorKind::TypeError, empty));
        }
        // 5. Let k be 0 (reduce) or len - 1 (reduceRight).
        let (mut index, step) = if ascending {
            (0.0, 1.0)
        } else {
            (length - 1.0, -1.0)
        };
        // 6. Let accumulator be undefined.
        let accumulator = self.keep(Value::Undefined);
        match initial {
            // 7. If initialValue is present, set accumulator to initialValue.
            Some(initial) => self.set_kept(accumulator, initial)?,
            // 8. Else, the first element present, searched while k is in range.
            None => {
                let mut present = false;
                while !present && (0.0..length).contains(&index) {
                    self.charge_steps(1)?;
                    // 8.b.i, ii. Let Pk be ! ToString(𝔽(k)); set kPresent to ? HasProperty(O, Pk).
                    let key = self.index_key(index)?;
                    present = self.has_property(object, key)?;
                    // 8.b.iii. If kPresent is true, set accumulator to ? Get(O, Pk).
                    if present {
                        let key = self.index_key(index)?;
                        let value = self.get(object, key)?;
                        self.set_kept(accumulator, value)?;
                    }
                    // 8.b.iv. Set k to k + 1 (reduce) or k - 1 (reduceRight).
                    index += step;
                }
                // 8.c. If kPresent is false, throw a TypeError exception.
                if !present {
                    return Err(self.throw_error(ErrorKind::TypeError, empty));
                }
            }
        }
        // 9. Repeat, while k is in range.
        while (0.0..length).contains(&index) {
            self.charge_steps(1)?;
            // a, b. Let Pk be ! ToString(𝔽(k)); let kPresent be ? HasProperty(O, Pk).
            let key = self.index_key(index)?;
            if self.has_property(object, key)? {
                // c.i. Let kValue be ? Get(O, Pk).
                let key = self.index_key(index)?;
                let value = self.get(object, key)?;
                // c.ii. Set accumulator to ? Call(callback, undefined, « accumulator, kValue,
                // 𝔽(k), O »).
                let previous = self.kept_value(accumulator)?;
                let next = self.call_value(
                    callback,
                    Value::Undefined,
                    &[previous, value, Value::Number(index), Value::Object(object)],
                )?;
                self.set_kept(accumulator, next)?;
            }
            // d. Set k to k + 1 (reduce) or k - 1 (reduceRight).
            index += step;
        }
        // 10. Return accumulator.
        self.kept_value(accumulator)
    }

    /// Steps 2 to the end of `flat` (`#sec-array.prototype.flat`) and, with `map`, of `flatMap`
    /// (`#sec-array.prototype.flatmap`), with O kept: the depth or the mapper, then A by
    /// ArraySpeciesCreate, kept, then FlattenIntoArray (`#sec-flattenintoarray`) from O into A
    /// at 0 on the explicit stack, whose charge is credited when it ends, however it ends.
    fn array_flat(&mut self, object: ObjectRef, args: &[Value], map: bool) -> Completion<Value> {
        // 2. Let sourceLen be ? LengthOfArrayLike(O).
        let length = self.length_of_array_like(object)?;
        let (depth, mapper) = if map {
            // flatMap 3. If IsCallable(mapperFunc) is false, throw a TypeError exception.
            let mapper = argument(args, 0);
            self.require_callable(
                mapper,
                "Array.prototype.flatMap: the mapper is not a function",
            )?;
            (1.0, Some((mapper, argument(args, 1))))
        } else {
            // flat 3, 4. Let depthNum be 1; if depth is not undefined, ToIntegerOrInfinity of
            // it, and 0 if that is below 0.
            let depth = match argument(args, 0) {
                Value::Undefined => 1.0,
                depth => self.to_integer_or_infinity(depth)?.max(0.0),
            };
            (depth, None)
        };
        // 5 (flat), 4 (flatMap). Let A be ? ArraySpeciesCreate(O, 0).
        let array = self.array_species_create(object, 0.0)?;
        self.keep(Value::Object(array));
        // 6 (flat), 5 (flatMap). Perform ? FlattenIntoArray(A, O, sourceLen, 0, depthNum
        // [ , mapperFunc, thisArg ]).
        let mut stack = Vec::new();
        let mut charged = 0;
        let mark = self.kept.len();
        let flattened = self.flatten_into_array(
            array,
            Open {
                source: object,
                length,
                index: 0.0,
                depth,
                mark,
            },
            mapper,
            &mut stack,
            &mut charged,
        );
        self.heap.accounting.credit(charged);
        flattened?;
        // 7 (flat), 6 (flatMap). Return A.
        Ok(Value::Object(array))
    }

    /// Open `open` on the explicit stack of FlattenIntoArray: charged to the memory budget
    /// first, recorded in `charged` until it is closed.
    fn open_source(
        &mut self,
        stack: &mut Vec<Open>,
        charged: &mut u64,
        open: Open,
    ) -> Completion<()> {
        if self.heap.accounting.charge(OPEN_BYTES).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        *charged = charged.saturating_add(OPEN_BYTES);
        if stack.try_reserve(1).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        stack.push(open);
        Ok(())
    }

    /// FlattenIntoArray ( target, source, sourceLength, start, depth [ , mapperFunc [ , thisArg
    /// ] ] ) (`#sec-flattenintoarray`) with start 0, as a loop over `stack`: the entry on top is
    /// the call of the clause that runs, a nested Array to flatten is a new entry (the
    /// recursive call of step 3.c.v.4), and an entry whose sourceIndex reached its
    /// sourceLength is closed (its call returns). targetIndex is one counter for all of them,
    /// as each recursive call answers it to its caller. The mapper is called for the elements
    /// of the outermost source only: the recursive call passes none.
    fn flatten_into_array(
        &mut self,
        target: ObjectRef,
        outermost: Open,
        mapper: Option<(Value, Value)>,
        stack: &mut Vec<Open>,
        charged: &mut u64,
    ) -> Completion<()> {
        // 2. Let targetIndex be start.
        let mut target_index = 0.0;
        self.open_source(stack, charged, outermost)?;
        while let Some(&Open {
            source,
            length,
            index,
            depth,
            mark,
        }) = stack.last()
        {
            // 4. Return targetIndex: the call of this source ends.
            if index >= length {
                stack.pop();
                self.release(mark);
                self.heap.accounting.credit(OPEN_BYTES);
                *charged = charged.saturating_sub(OPEN_BYTES);
                continue;
            }
            self.charge_steps(1)?;
            // 3.d. Set sourceIndex to sourceIndex + 1 (done first: the steps below may open a
            // new entry above this one).
            if let Some(top) = stack.last_mut() {
                top.index = index + 1.0;
            }
            // 3.a, b. Let P be ! ToString(𝔽(sourceIndex)); let exists be ? HasProperty(source, P).
            let key = self.index_key(index)?;
            if !self.has_property(source, key)? {
                continue;
            }
            // 3.c.i. Let element be ? Get(source, P). It is kept: the mapper and the steps
            // below may run code, and it is read or written after.
            let key = self.index_key(index)?;
            let mut element = self.get(source, key)?;
            let element_mark = self.keep(element);
            // 3.c.ii. If mapperFunc is present, set element to ? Call(mapperFunc, thisArg,
            // « element, sourceIndex, source »).
            if let Some((mapper, this_arg)) = mapper
                && stack.len() == 1
            {
                element = self.call_value(
                    mapper,
                    this_arg,
                    &[element, Value::Number(index), Value::Object(source)],
                )?;
                self.set_kept(element_mark, element)?;
            }
            // 3.c.iii, iv. shouldFlatten is false, or ? IsArray(element) if depth > 0.
            let nested = match element {
                Value::Object(nested) if depth > 0.0 && self.is_array(element)? => Some(nested),
                _ => None,
            };
            match nested {
                // 3.c.v. If shouldFlatten is true: newDepth is +∞ if depth is, else depth - 1;
                // let elementLen be ? LengthOfArrayLike(element); then the recursive call,
                // FlattenIntoArray(target, element, elementLen, targetIndex, newDepth), whose
                // source stays kept at `element_mark` until it is closed.
                Some(nested) => {
                    let new_depth = if depth == f64::INFINITY {
                        f64::INFINITY
                    } else {
                        depth - 1.0
                    };
                    let nested_length = self.length_of_array_like(nested)?;
                    self.open_source(
                        stack,
                        charged,
                        Open {
                            source: nested,
                            length: nested_length,
                            index: 0.0,
                            depth: new_depth,
                            mark: element_mark,
                        },
                    )?;
                }
                // 3.c.vi. Else: if targetIndex ≥ 2^53 - 1, throw a TypeError exception; ?
                // CreateDataPropertyOrThrow(target, ! ToString(𝔽(targetIndex)), element); set
                // targetIndex to targetIndex + 1.
                None => {
                    if target_index >= MAX_SAFE_LENGTH {
                        return Err(self.throw_error(
                            ErrorKind::TypeError,
                            "FlattenIntoArray would write an index of 2^53 - 1 or above",
                        ));
                    }
                    let key = self.index_key(target_index)?;
                    self.create_data_property_or_throw(target, key, element)?;
                    target_index += 1.0;
                    self.release(element_mark);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::host::agent::Agent;
    use crate::host::hooks::test_host::{NoHooks, agent_with_realm, enter};
    use crate::host::outcome::{Abrupt, Completion};
    use crate::host::{Cx, HostCall};
    use crate::object::ordinary::OrdinaryData;
    use crate::object::property::{Descriptor, Key};
    use crate::ops::objects::IntegrityLevel;
    use crate::realm::intrinsics::Intrinsic;
    use crate::value::{ObjectRef, Value};

    /// What a species constructor of these tests does to the object it makes before it answers
    /// it.
    #[derive(Clone, Copy)]
    enum Made {
        /// Nothing: an ordinary extensible object.
        Plain,
        /// [[PreventExtensions]].
        Closed,
        /// An own "0", then SetIntegrityLevel frozen.
        Frozen,
    }

    /// A species constructor: it records the length it is given as its callee's "seen"
    /// property and answers a new ordinary object, changed as `made` says.
    fn species_making(
        cx: &mut Cx<'_, NoHooks>,
        call: HostCall<'_>,
        made: Made,
    ) -> Completion<Value> {
        let agent = &mut *cx.agent;
        let key = agent.name_key("seen")?;
        let length = call.args.first().copied().unwrap_or(Value::Undefined);
        agent.create_data_property(call.callee, key, length)?;
        let prototype = agent.current_intrinsic(Intrinsic::ObjectPrototype)?;
        let object = agent.ordinary_object_create(Some(prototype), OrdinaryData::Plain)?;
        match made {
            Made::Plain => {}
            Made::Closed => {
                agent.prevent_extensions(object)?;
            }
            Made::Frozen => {
                agent.create_data_property(object, Key::Index(0), Value::Null)?;
                agent.set_integrity_level(object, IntegrityLevel::Frozen)?;
            }
        }
        Ok(Value::Object(object))
    }

    fn plain_species(cx: &mut Cx<'_, NoHooks>, call: HostCall<'_>) -> Completion<Value> {
        species_making(cx, call, Made::Plain)
    }

    fn closed_species(cx: &mut Cx<'_, NoHooks>, call: HostCall<'_>) -> Completion<Value> {
        species_making(cx, call, Made::Closed)
    }

    fn frozen_species(cx: &mut Cx<'_, NoHooks>, call: HostCall<'_>) -> Completion<Value> {
        species_making(cx, call, Made::Frozen)
    }

    /// A callback that selects every element.
    fn select_all(_cx: &mut Cx<'_, NoHooks>, _call: HostCall<'_>) -> Completion<Value> {
        Ok(Value::Boolean(true))
    }

    /// A mapper that answers its value.
    fn identity(_cx: &mut Cx<'_, NoHooks>, call: HostCall<'_>) -> Completion<Value> {
        Ok(call.args.first().copied().unwrap_or(Value::Undefined))
    }

    /// `Array.prototype[name]`.
    fn method(agent: &mut Agent<NoHooks>, prototype: ObjectRef, name: &str) -> Value {
        let key = agent.name_key(name).unwrap();
        agent.get(prototype, key).unwrap()
    }

    /// A host function of the realm `handle`, kept.
    fn host_function(
        agent: &mut Agent<NoHooks>,
        handle: crate::heap::arena::Handle,
        name: &str,
        steps: crate::host::HostFn<NoHooks>,
        constructor: bool,
    ) -> ObjectRef {
        let function = agent
            .create_host_function(handle, name, 0, steps, constructor, None)
            .unwrap();
        agent.keep(Value::Object(function));
        function
    }

    /// A kept Array of the current realm with `elements` (`None` a hole) whose own
    /// "constructor" is `owner`.
    fn array_owned_by(
        agent: &mut Agent<NoHooks>,
        elements: &[Option<Value>],
        owner: ObjectRef,
    ) -> ObjectRef {
        let array = agent.array_create(elements.len() as f64, None).unwrap();
        agent.keep(Value::Object(array));
        for (index, element) in elements.iter().enumerate() {
            if let Some(element) = *element {
                let index = Key::Index(u32::try_from(index).unwrap());
                agent.create_data_property(array, index, element).unwrap();
            }
        }
        let key = agent.name_key("constructor").unwrap();
        agent
            .create_data_property(array, key, Value::Object(owner))
            .unwrap();
        array
    }

    /// `#sec-arrayspeciescreate`, step 5, the part that needs @@species, which no script names
    /// before slice 6: `filter` (`#sec-array.prototype.filter`), `flat`
    /// (`#sec-array.prototype.flat`) and `flatMap` (`#sec-array.prototype.flatmap`) of an Array
    /// whose constructor has an own @@species constructor get their result from it, called with
    /// the length 0, and fill it from index 0 in the order of the source, skipping holes; a
    /// @@species of null makes an Array; one that is not a constructor throws a TypeError.
    #[test]
    fn filter_uses_the_species_and_keeps_the_order() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let prototype = agent.intrinsic(handle, Intrinsic::ArrayPrototype).unwrap();
        let species_key = Key::Symbol(agent.well_known().unwrap().species);
        let species = host_function(&mut agent, handle, "Species", plain_species, true);
        let owner = host_function(&mut agent, handle, "Owner", plain_species, true);
        let select = host_function(&mut agent, handle, "select", select_all, false);
        let same = host_function(&mut agent, handle, "identity", identity, false);
        let [a, c, d] = ["a", "c", "d"].map(|text| {
            let string = Value::String(agent.new_string_str(text).unwrap());
            agent.keep(string);
            string
        });
        let inner = agent.array_create(1.0, None).unwrap();
        agent.keep(Value::Object(inner));
        agent.create_data_property(inner, Key::Index(0), d).unwrap();
        let source = array_owned_by(
            &mut agent,
            &[Some(a), None, Some(c), Some(Value::Object(inner))],
            owner,
        );
        agent
            .create_data_property(owner, species_key, Value::Object(species))
            .unwrap();
        let calls = [
            ("filter", vec![Value::Object(select)], Value::Object(inner)),
            ("flat", vec![], d),
            ("flatMap", vec![Value::Object(same)], d),
        ];
        for (name, args, third) in &calls {
            let seen_key = agent.name_key("seen").unwrap();
            agent
                .create_data_property(species, seen_key, Value::Undefined)
                .unwrap();
            let function = method(&mut agent, prototype, name);
            let made = agent.call_value(function, Value::Object(source), args);
            let Ok(Value::Object(made)) = made else {
                panic!("{name} answered {made:?}");
            };
            let seen_key = agent.name_key("seen").unwrap();
            assert_eq!(
                agent.get(species, seen_key),
                Ok(Value::Number(0.0)),
                "{name}"
            );
            assert_eq!(agent.is_array(Value::Object(made)), Ok(false), "{name}");
            assert_eq!(agent.get(made, Key::Index(0)), Ok(a), "{name}");
            assert_eq!(agent.get(made, Key::Index(1)), Ok(c), "{name}");
            assert_eq!(agent.get(made, Key::Index(2)), Ok(*third), "{name}");
            assert_eq!(agent.has_property(made, Key::Index(3)), Ok(false), "{name}");
        }

        let species_property = |value| Descriptor::data(value, true, true, true);
        agent
            .define_own_property(owner, species_key, species_property(Value::Null))
            .unwrap();
        for (name, args, _) in &calls {
            let function = method(&mut agent, prototype, name);
            let made = agent.call_value(function, Value::Object(source), args);
            let Ok(Value::Object(made)) = made else {
                panic!("{name} answered {made:?}");
            };
            assert_eq!(agent.is_array(Value::Object(made)), Ok(true), "{name}");
            assert_eq!(agent.get(made, Key::Index(1)), Ok(c), "{name}");
        }
        agent
            .define_own_property(owner, species_key, species_property(Value::Object(select)))
            .unwrap();
        for (name, args, _) in &calls {
            let function = method(&mut agent, prototype, name);
            let made = agent.call_value(function, Value::Object(source), args);
            assert!(matches!(made, Err(Abrupt::Throw(_))), "{name}: {made:?}");
        }
    }

    /// CreateDataPropertyOrThrow (`#sec-createdatapropertyorthrow`) into the result of the
    /// species: `filter`, `flat` and `flatMap` throw a TypeError when the object the @@species
    /// constructor makes is not extensible ([[PreventExtensions]]) or holds a "0" that cannot
    /// be defined (SetIntegrityLevel frozen); with no element to write they answer that object.
    #[test]
    fn a_target_that_cannot_take_an_element_throws() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let prototype = agent.intrinsic(handle, Intrinsic::ArrayPrototype).unwrap();
        let species_key = Key::Symbol(agent.well_known().unwrap().species);
        let owner = host_function(&mut agent, handle, "Owner", plain_species, true);
        let select = host_function(&mut agent, handle, "select", select_all, false);
        let same = host_function(&mut agent, handle, "identity", identity, false);
        let full = array_owned_by(&mut agent, &[Some(Value::Number(1.0))], owner);
        let empty = array_owned_by(&mut agent, &[], owner);
        for steps in [closed_species, frozen_species] {
            let species = host_function(&mut agent, handle, "Species", steps, true);
            agent
                .define_own_property(
                    owner,
                    species_key,
                    Descriptor::data(Value::Object(species), true, true, true),
                )
                .unwrap();
            for (name, args) in [
                ("filter", vec![Value::Object(select)]),
                ("flat", vec![]),
                ("flatMap", vec![Value::Object(same)]),
            ] {
                let function = method(&mut agent, prototype, name);
                let thrown = agent.call_value(function, Value::Object(full), &args);
                let Err(Abrupt::Throw(Value::Object(error))) = thrown else {
                    panic!("{name} answered {thrown:?}");
                };
                let name_key = agent.name_key("name").unwrap();
                let error_name = agent.get(error, name_key).unwrap();
                let Value::String(error_name) = error_name else {
                    panic!("{error_name:?}");
                };
                let units = agent.string_units(error_name).unwrap();
                assert_eq!(String::from_utf16_lossy(&units), "TypeError", "{name}");
                let function = method(&mut agent, prototype, name);
                let made = agent.call_value(function, Value::Object(empty), &args);
                assert!(matches!(made, Ok(Value::Object(_))), "{name}: {made:?}");
            }
        }
    }
}
