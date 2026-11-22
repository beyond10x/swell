//! `Array.from ( items [ , mapper [ , thisArg ] ] )` (ECMA-262 23.1.2.1, `#sec-array.from`):
//! the iterable path through GetIteratorFromMethod (`#sec-getiteratorfrommethod`) and
//! IteratorStepValue (`#sec-iteratorstepvalue`), closed by IteratorClose
//! (`#sec-iteratorclose`) when the mapper or the definition of an element ends abruptly
//! (IfAbruptCloseIterator, `#sec-ifabruptcloseiterator`), and the array-like path. Written by
//! `story:array-from-and-object-from-entries`.
//!
//! The design is `docs/design/engine.md`, decision 3, "Iterators held by a builtin": the
//! Iterator Record's [[Iterator]] and [[NextMethod]], the method of `@@iterator` while the
//! constructor runs, the array-like object and the object being built are kept on the agent's
//! kept stack until the builtin returns, on every way out (item 1); the mapper, its this value
//! and `items` are held by the builtin's activation. Each loop charges one step per iteration
//! through `charge_steps` before its work, beside the steps the call path charges for `next`
//! and the mapper (items 2 and 3). A throw of `next`, `done` or `value` ends the builtin with no
//! close, and a termination closes nothing (item 4).

use super::MAX_SAFE_LENGTH;
use crate::builtins::object::argument;
use crate::host::agent::Agent;
use crate::host::outcome::{Abrupt, Completion};
use crate::host::{ErrorKind, Host};
use crate::object::property::Key;
use crate::ops::iteration::IteratorRecord;
use crate::value::{ObjectRef, Value};

impl<H: Host> Agent<H> {
    /// `Array.from ( items [ , mapper [ , thisArg ] ] )` (`#sec-array.from`); what the steps
    /// keep is released on every way out.
    pub(crate) fn array_from(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        let mark = self.kept.len();
        let made = self.array_from_steps(this, args);
        self.release(mark);
        made
    }

    /// Steps 1 to 6 of `Array.from`, choosing the path. `items`, `mapper` and `thisArg` are
    /// held by the activation of `Array.from`.
    fn array_from_steps(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        let items = argument(args, 0);
        let mapper = argument(args, 1);
        // 1. Let ctor be the this value.
        // 2. If mapper is undefined, let mapping be false. 3. Else, if IsCallable(mapper) is
        // false, throw a TypeError exception; let mapping be true.
        let mapper = match mapper {
            Value::Undefined => None,
            mapper if self.is_callable(mapper)? => Some(mapper),
            _ => {
                return Err(self.throw_error(
                    ErrorKind::TypeError,
                    "Array.from: the mapper is not a function",
                ));
            }
        };
        let mapping = Mapping {
            mapper,
            this_arg: argument(args, 2),
        };
        // 4. Let usingIterator be ? GetMethod(items, %Symbol.iterator%).
        let Some(symbols) = self.well_known() else {
            return Err(self.fault());
        };
        match self.get_method(items, Key::Symbol(symbols.iterator))? {
            // 5. If usingIterator is not undefined: the method is kept while ctor runs.
            Some(method) => {
                self.keep(method);
                self.array_from_iterable(this, items, method, mapping)
            }
            // 6. NOTE: items is not iterable so assume it is an array-like object.
            None => self.array_from_array_like(this, items, mapping),
        }
    }

    /// Step 5 of `Array.from`: the iterable path.
    fn array_from_iterable(
        &mut self,
        constructor: Value,
        items: Value,
        method: Value,
        mapping: Mapping,
    ) -> Completion<Value> {
        // 5.a, 5.b. If IsConstructor(ctor), let array be ? Construct(ctor); else ! ArrayCreate(0).
        let array = self.construct_or_array_create(constructor, None)?;
        self.keep(Value::Object(array));
        // 5.c. Let iteratorRecord be ? GetIteratorFromMethod(items, usingIterator).
        let mut record = self.get_iterator_from_method(items, method)?;
        self.keep(Value::Object(record.iterator));
        self.keep(record.next_method);
        // 5.d. Let k be 0. 5.e. Repeat,
        let mut index = 0.0;
        loop {
            self.charge_steps(1)?;
            // 5.e.i. If k ≥ 2^53 - 1, return ? IteratorClose(iteratorRecord, a TypeError).
            if index >= MAX_SAFE_LENGTH {
                let error = self.throw_error(
                    ErrorKind::TypeError,
                    "Array.from would make a length above 2^53 - 1",
                );
                return Err(self.close_iterator_after(&record, error));
            }
            // 5.e.ii. Let propertyKey be ! ToString(𝔽(k)): made after the calls, which may have
            // collected the atom of an index above 2^32 - 2.
            // 5.e.iii. Let next be ? IteratorStepValue(iteratorRecord).
            let Some(next) = self.iterator_step_value(&mut record)? else {
                // 5.e.iv. If next is done, perform ? Set(array, "length", 𝔽(k), true) and
                // return array.
                let key = self.name_key("length")?;
                self.set(array, key, Value::Number(index), true)?;
                return Ok(Value::Object(array));
            };
            // 5.e.v to 5.e.viii, with IfAbruptCloseIterator of the mapped value and of the
            // definition.
            if let Err(abrupt) = self.array_from_element(array, next, index, mapping) {
                return Err(self.close_iterator_after(&record, abrupt));
            }
            // 5.e.ix. Set k to k + 1.
            index += 1.0;
        }
    }

    /// Steps 5.e.v to 5.e.viii of `Array.from`, and 11.c and 11.d of the array-like path: the
    /// mapped value of `value` at `index`, defined on `array`. The call of the mapper holds
    /// `value` in its activation; the key is made after it.
    fn array_from_element(
        &mut self,
        array: ObjectRef,
        value: Value,
        index: f64,
        mapping: Mapping,
    ) -> Completion<()> {
        // If mapping is true, let mappedValue be Call(mapper, thisArg, « next, 𝔽(k) »);
        // otherwise mappedValue is next.
        let mapped = match mapping.mapper {
            Some(mapper) => {
                self.call_value(mapper, mapping.this_arg, &[value, Value::Number(index)])?
            }
            None => value,
        };
        // Let defineStatus be CreateDataPropertyOrThrow(array, propertyKey, mappedValue). No code
        // runs between the call and the definition, so the mapped value needs no keep.
        let key = self.index_key(index)?;
        self.create_data_property_or_throw(array, key, mapped)
    }

    /// Steps 7 to 13 of `Array.from`: the array-like path.
    fn array_from_array_like(
        &mut self,
        constructor: Value,
        items: Value,
        mapping: Mapping,
    ) -> Completion<Value> {
        // 7. Let arrayLike be ! ToObject(items). It may be new, and the steps after run code.
        let array_like = self.to_object(items)?;
        self.keep(Value::Object(array_like));
        // 8. Let len be ? LengthOfArrayLike(arrayLike).
        let length = self.length_of_array_like(array_like)?;
        // 9. If IsConstructor(ctor), let array be ? Construct(ctor, « 𝔽(len) »); else ?
        // ArrayCreate(len).
        let array = self.construct_or_array_create(constructor, Some(length))?;
        self.keep(Value::Object(array));
        // 10. Let k be 0. 11. Repeat, while k < len,
        let mut index = 0.0;
        while index < length {
            self.charge_steps(1)?;
            // 11.a, 11.b. Let kValue be ? Get(arrayLike, ! ToString(𝔽(k))). Nothing runs
            // between the Get and the call of the mapper, which holds kValue.
            let key = self.index_key(index)?;
            let value = self.get(array_like, key)?;
            // 11.c, 11.d. The mapped value, then ? CreateDataPropertyOrThrow.
            self.array_from_element(array, value, index, mapping)?;
            // 11.e. Set k to k + 1.
            index += 1.0;
        }
        // 12. Perform ? Set(array, "length", 𝔽(len), true). 13. Return array.
        let key = self.name_key("length")?;
        self.set(array, key, Value::Number(length), true)?;
        Ok(Value::Object(array))
    }

    /// Steps 5.a and 5.b, and 9, of `Array.from`: Construct(ctor) with no argument, or with
    /// `length` when it is given, when ctor is a constructor; ArrayCreate(length) otherwise.
    fn construct_or_array_create(
        &mut self,
        constructor: Value,
        length: Option<f64>,
    ) -> Completion<ObjectRef> {
        match constructor {
            Value::Object(constructor) if self.has_construct(constructor)? => {
                let args = length.map(Value::Number);
                match self.construct_value(constructor, args.as_slice(), None)? {
                    Value::Object(made) => Ok(made),
                    // Construct answers an Object, or throws.
                    _ => Err(self.fault()),
                }
            }
            _ => self.array_create(length.unwrap_or(0.0), None),
        }
    }

    /// IfAbruptCloseIterator (`#sec-ifabruptcloseiterator`) of the abrupt completion `abrupt`:
    /// IteratorClose (`#sec-iteratorclose`) with it, which answers it whatever `return` throws
    /// or answers, and calls no `return` for a termination. The caller keeps [[Iterator]].
    pub(crate) fn close_iterator_after(
        &mut self,
        record: &IteratorRecord,
        abrupt: Abrupt,
    ) -> Abrupt {
        match self.iterator_close(record, Err(abrupt)) {
            Err(abrupt) => abrupt,
            // IteratorClose with an abrupt completion answers an abrupt completion.
            Ok(_) => self.fault(),
        }
    }
}

/// The mapper of `Array.from` when mapping is true, and its this value; both are held by the
/// activation of `Array.from`.
#[derive(Clone, Copy)]
struct Mapping {
    mapper: Option<Value>,
    this_arg: Value,
}
