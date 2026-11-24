//! `Array.fromAsync ( items [ , mapper [ , thisArg ] ] )` (ECMA-262 23.1.2.2,
//! `#sec-array.fromasync`), written by `story:array-from-async`: an async function of the
//! draft ("This async function performs the following steps when called"), run as a built-in
//! async function (`docs/design/engine.md`, decision 3, "A built-in async function";
//! `object/async_builtin.rs`). Its state lies in the box of its activation ([`FromAsync`]): the
//! step it reached, the Iterator Record or the array-like and its length, the array, k, the
//! mapper and thisArg, and the completion AsyncIteratorClose returns.
//!
//! - The branch of an iterable: GetIterator with kind async (`#sec-getiterator`), or with kind
//!   sync and CreateAsyncFromSyncIterator (`#sec-createasyncfromsynciterator`); each result of
//!   `next` awaited, the mapper's result awaited, CreateDataPropertyOrThrow, and
//!   IfAbruptCloseAsyncIterator (`#sec-ifabruptcloseasynciterator`) with AsyncIteratorClose
//!   (`#sec-asynciteratorclose`), which awaits the result of `return`, on every abrupt
//!   completion the draft marks so; `length` set at the end.
//! - The branch of an array-like: each element awaited, in order, then the mapper's result.
//!
//! Every abrupt step the draft marks with `?` is a throw completion of the steps, which rejects
//! the promise of the call (AsyncBlockStart, `#sec-asyncblockstart`). Every turn of both loops
//! awaits at least once, so no run of the steps loops: the call runs them to the first Await,
//! and each continuation from one Await to the next (item 2). A call into code (`next`,
//! `return`, the mapper, a getter, a constructor) is a re-entry through the agent's call path;
//! the activation, kept while the steps run, holds every value they need across it, and a
//! value read from a call is kept until the box or an Await holds it.

use super::MAX_SAFE_LENGTH;
use crate::builtins::object::argument;
use crate::heap::collect::Mark;
use crate::host::agent::Agent;
use crate::host::outcome::{Abrupt, Completion};
use crate::host::{ErrorKind, Host};
use crate::object::async_builtin::{AsyncBuiltinSteps, Reached};
use crate::object::promise::ReactionType;
use crate::object::property::Key;
use crate::ops::iteration::IteratorRecord;
use crate::value::{ObjectRef, Value};

/// The Await of the steps an activation of `Array.fromAsync` waits at, which decides where a
/// continuation goes on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FromAsyncStep {
    /// Before the first Await.
    Start,
    /// `Set nextResult to ? Await(nextResult).`
    NextResult,
    /// `Set mappedValue to Completion(Await(mappedValue)).` of the branch of an iterable.
    Mapped,
    /// `Set kValue to ? Await(kValue).` of the branch of an array-like.
    Element,
    /// `Set mappedValue to ? Await(mappedValue).` of the branch of an array-like.
    MappedElement,
    /// The Await of the result of `return` in AsyncIteratorClose (step 4.d).
    Closed,
}

/// What `Array.fromAsync` reads its values from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Input {
    /// Not chosen yet.
    None,
    /// iteratorRecord, an async iterator or an Async-from-Sync Iterator.
    Iterator(IteratorRecord),
    /// arrayLike and its length.
    ArrayLike { object: ObjectRef, length: f64 },
}

/// The state of a call of `Array.fromAsync`, in the box of its activation.
pub(crate) struct FromAsync {
    pub(crate) step: FromAsyncStep,
    pub(crate) input: Input,
    /// array, once made.
    pub(crate) array: Option<ObjectRef>,
    /// k.
    pub(crate) index: f64,
    /// mapper when mapping is true, and thisArg.
    pub(crate) mapper: Option<Value>,
    pub(crate) this_arg: Value,
    /// The throw completion AsyncIteratorClose returns once the result of `return` is awaited.
    pub(crate) error: Value,
}

impl FromAsync {
    /// Every value the state holds.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        match self.input {
            Input::None => {}
            Input::Iterator(record) => {
                mark.object(record.iterator);
                mark.value(record.next_method);
            }
            Input::ArrayLike { object, .. } => mark.object(object),
        }
        if let Some(array) = self.array {
            mark.object(array);
        }
        if let Some(mapper) = self.mapper {
            mark.value(mapper);
        }
        mark.value(self.this_arg);
        mark.value(self.error);
    }

    /// A state whose values are `object`, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test(object: ObjectRef) -> FromAsync {
        FromAsync {
            step: FromAsyncStep::Element,
            input: Input::ArrayLike {
                object,
                length: 1.0,
            },
            array: Some(object),
            index: 0.0,
            mapper: Some(Value::Object(object)),
            this_arg: Value::Object(object),
            error: Value::Object(object),
        }
    }
}

impl<H: Host> Agent<H> {
    /// `Array.fromAsync ( items [ , mapper [ , thisArg ] ] )`: its activation made with the
    /// promise of the call, its steps run to the first Await or their end, and the promise
    /// answered. `this`, `items` and `mapper` are held by the activation of the call.
    pub(crate) fn array_from_async(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        let state = FromAsync {
            step: FromAsyncStep::Start,
            input: Input::None,
            array: None,
            index: 0.0,
            mapper: None,
            this_arg: argument(args, 2),
            error: Value::Undefined,
        };
        let items = argument(args, 0);
        let mapper = argument(args, 1);
        self.start_async_builtin(AsyncBuiltinSteps::FromAsync(state), |agent, activation| {
            agent.array_from_async_steps(activation, this, items, mapper)
        })
    }

    /// The state of the activation `activation` of `Array.fromAsync`, to change.
    fn array_from_async_mut(&mut self, activation: ObjectRef) -> Completion<&mut FromAsync> {
        let record = self.async_builtin_mut(activation)?;
        match record.steps.as_mut() {
            AsyncBuiltinSteps::FromAsync(state) => Ok(state),
        }
    }

    /// The steps of the call, from step 1 to the first Await.
    fn array_from_async_steps(
        &mut self,
        activation: ObjectRef,
        constructor: Value,
        items: Value,
        mapper: Value,
    ) -> Completion<Reached> {
        // 1. Let ctor be the this value. 2. Let mapping be false. 3. If mapper is not
        // undefined, then a. If IsCallable(mapper) is false, throw a TypeError exception. b. Set
        // mapping to true.
        let mapper = match mapper {
            Value::Undefined => None,
            mapper if self.is_callable(mapper)? => Some(mapper),
            _ => {
                return Err(self.throw_error(
                    ErrorKind::TypeError,
                    "Array.fromAsync: the mapper is not a function",
                ));
            }
        };
        self.array_from_async_mut(activation)?.mapper = mapper;
        // 4. Let iteratorRecord be undefined. 5. Let usingAsyncIterator be ? GetMethod(items,
        // %Symbol.asyncIterator%).
        let Some(symbols) = self.well_known() else {
            return Err(self.fault());
        };
        let record = match self.get_method(items, Key::Symbol(symbols.async_iterator))? {
            // 7. Else, set iteratorRecord to ? GetIteratorFromMethod(items, usingAsyncIterator).
            // The call holds the method.
            Some(method) => Some(self.get_iterator_from_method(items, method)?),
            // 6. If usingAsyncIterator is undefined, then a. Let usingSyncIterator be ?
            // GetMethod(items, %Symbol.iterator%). b. If usingSyncIterator is not undefined, set
            // iteratorRecord to CreateAsyncFromSyncIterator(? GetIteratorFromMethod(items,
            // usingSyncIterator)).
            None => match self.get_method(items, Key::Symbol(symbols.iterator))? {
                Some(method) => {
                    let sync = self.get_iterator_from_method(items, method)?;
                    Some(self.create_async_from_sync_iterator(sync)?)
                }
                None => None,
            },
        };
        match record {
            // 8. If iteratorRecord is not undefined, then
            Some(record) => {
                // The box holds the record from here on.
                self.array_from_async_mut(activation)?.input = Input::Iterator(record);
                // a. If IsConstructor(ctor) is true, then i. Let array be ? Construct(ctor). b.
                // Else, i. Let array be ! ArrayCreate(0).
                let array = self.array_from_async_construct(constructor, None)?;
                self.array_from_async_mut(activation)?.array = Some(array);
                // c. Let k be 0. d. Repeat,
                self.array_from_async_next(activation)
            }
            // 9. Else,
            None => self.array_from_async_array_like(activation, constructor, items),
        }
    }

    /// Steps 8.a and 9.d of `Array.fromAsync`: Construct(ctor) with no argument, or with
    /// `length` when it is given, when ctor is a constructor; ArrayCreate(length) otherwise.
    fn array_from_async_construct(
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

    /// The array the state of `activation` holds.
    fn array_from_async_array(&mut self, activation: ObjectRef) -> Completion<ObjectRef> {
        match self.array_from_async_mut(activation)?.array {
            Some(array) => Ok(array),
            None => Err(self.fault()),
        }
    }

    /// The Iterator Record the state of `activation` holds.
    fn array_from_async_record(&mut self, activation: ObjectRef) -> Completion<IteratorRecord> {
        match self.array_from_async_mut(activation)?.input {
            Input::Iterator(record) => Ok(record),
            Input::None | Input::ArrayLike { .. } => Err(self.fault()),
        }
    }

    /// The Await of `value` at `step`: the activation suspended there, or the throw of
    /// PromiseResolve.
    fn array_from_async_await(
        &mut self,
        activation: ObjectRef,
        step: FromAsyncStep,
        value: Value,
    ) -> Completion<Reached> {
        self.array_from_async_mut(activation)?.step = step;
        self.async_builtin_await(activation, value)?;
        Ok(Reached::Await)
    }

    /// A turn of the loop of step 8.d, up to its Await of the result of `next`.
    fn array_from_async_next(&mut self, activation: ObjectRef) -> Completion<Reached> {
        let index = self.array_from_async_mut(activation)?.index;
        // i. If k ≥ 2^53 - 1, then 1. Let error be ThrowCompletion(a newly created TypeError
        // object). 2. Return ? AsyncIteratorClose(iteratorRecord, error).
        if index >= MAX_SAFE_LENGTH {
            let error = match self.throw_error(
                ErrorKind::TypeError,
                "Array.fromAsync would make a length above 2^53 - 1",
            ) {
                Abrupt::Throw(error) => error,
                abrupt => return Err(abrupt),
            };
            return self.array_from_async_close(activation, error);
        }
        // ii. Let propertyKey be ! ToString(𝔽(k)): made when the value is defined. iii. Let
        // nextResult be ? Call(iteratorRecord.[[NextMethod]], iteratorRecord.[[Iterator]]).
        let record = self.array_from_async_record(activation)?;
        let next_result =
            self.call_value(record.next_method, Value::Object(record.iterator), &[])?;
        // iv. Set nextResult to ? Await(nextResult).
        self.array_from_async_await(activation, FromAsyncStep::NextResult, next_result)
    }

    /// The continuation of the steps of `activation` from the Await it waited at, with the
    /// value it was fulfilled with (`Fulfill`) or the reason it was rejected with (`Reject`).
    pub(crate) fn array_from_async_continue(
        &mut self,
        activation: ObjectRef,
        kind: ReactionType,
        value: Value,
    ) -> Completion<Reached> {
        let step = self.array_from_async_mut(activation)?.step;
        match (step, kind) {
            // The Await of `? Await(...)` with a rejection: its throw completion.
            (
                FromAsyncStep::NextResult | FromAsyncStep::Element | FromAsyncStep::MappedElement,
                ReactionType::Reject,
            ) => Err(Abrupt::Throw(value)),
            (FromAsyncStep::NextResult, ReactionType::Fulfill) => {
                self.array_from_async_next_result(activation, value)
            }
            // IfAbruptCloseAsyncIterator(mappedValue, iteratorRecord).
            (FromAsyncStep::Mapped, ReactionType::Reject) => {
                self.array_from_async_close(activation, value)
            }
            (FromAsyncStep::Mapped, ReactionType::Fulfill) => {
                self.array_from_async_define(activation, value)
            }
            (FromAsyncStep::Element, ReactionType::Fulfill) => {
                self.array_from_async_element(activation, value)
            }
            (FromAsyncStep::MappedElement, ReactionType::Fulfill) => {
                self.array_from_async_define_element(activation, value)
            }
            // AsyncIteratorClose, step 5: completion is a throw completion, which it returns
            // whatever the Await of the result of `return` answered.
            (FromAsyncStep::Closed, _) => {
                let error = self.array_from_async_mut(activation)?.error;
                Err(Abrupt::Throw(error))
            }
            (FromAsyncStep::Start, _) => Err(self.fault()),
        }
    }

    /// Steps 8.d.v to 8.d.ix with the awaited result `result` of `next`.
    fn array_from_async_next_result(
        &mut self,
        activation: ObjectRef,
        result: Value,
    ) -> Completion<Reached> {
        // v. If nextResult is not an Object, throw a TypeError exception.
        let Value::Object(result) = result else {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "Array.fromAsync: the iterator result is not an object",
            ));
        };
        // The result is held by the activation of the function the job called, which this
        // continuation runs in.
        // vi. Let done be ? IteratorComplete(nextResult).
        if self.iterator_complete(result)? {
            // vii. If done is true, then 1. Perform ? Set(array, "length", 𝔽(k), true). 2.
            // Return array.
            let (array, index) = {
                let state = self.array_from_async_mut(activation)?;
                (state.array, state.index)
            };
            let Some(array) = array else {
                return Err(self.fault());
            };
            let key = self.name_key("length")?;
            self.set(array, key, Value::Number(index), true)?;
            return Ok(Reached::Return(Value::Object(array)));
        }
        // viii. Let nextValue be ? IteratorValue(nextResult).
        let next_value = self.iterator_value(result)?;
        let (mapper, this_arg, index) = {
            let state = self.array_from_async_mut(activation)?;
            (state.mapper, state.this_arg, state.index)
        };
        match mapper {
            // ix. If mapping is true, then 1. Let mappedValue be Completion(Call(mapper,
            // thisArg, « nextValue, 𝔽(k) »)). 2. IfAbruptCloseAsyncIterator(mappedValue,
            // iteratorRecord). 3. Set mappedValue to Completion(Await(mappedValue)). 4.
            // IfAbruptCloseAsyncIterator(mappedValue, iteratorRecord).
            Some(mapper) => {
                let mapped =
                    match self.call_value(mapper, this_arg, &[next_value, Value::Number(index)]) {
                        Ok(mapped) => mapped,
                        Err(Abrupt::Throw(error)) => {
                            return self.array_from_async_close(activation, error);
                        }
                        Err(abrupt) => return Err(abrupt),
                    };
                match self.array_from_async_await(activation, FromAsyncStep::Mapped, mapped) {
                    Err(Abrupt::Throw(error)) => self.array_from_async_close(activation, error),
                    reached => reached,
                }
            }
            // x. Else, let mappedValue be nextValue.
            None => {
                let mark = self.keep(next_value);
                let defined = self.array_from_async_define(activation, next_value);
                self.release(mark);
                defined
            }
        }
    }

    /// Steps 8.d.xi to 8.d.xiii with the mapped value `value`: CreateDataPropertyOrThrow with
    /// IfAbruptCloseAsyncIterator, k raised, and the next turn.
    fn array_from_async_define(
        &mut self,
        activation: ObjectRef,
        value: Value,
    ) -> Completion<Reached> {
        // xi. Let defineStatus be Completion(CreateDataPropertyOrThrow(array, propertyKey,
        // mappedValue)). xii. IfAbruptCloseAsyncIterator(defineStatus, iteratorRecord).
        match self.array_from_async_create_data_property(activation, value) {
            Ok(()) => {}
            Err(Abrupt::Throw(error)) => return self.array_from_async_close(activation, error),
            Err(abrupt) => return Err(abrupt),
        }
        // xiii. Set k to k + 1.
        let state = self.array_from_async_mut(activation)?;
        state.index += 1.0;
        self.array_from_async_next(activation)
    }

    /// CreateDataPropertyOrThrow(array, ! ToString(𝔽(k)), value): the key is made here, after
    /// the calls of the turn, which may have collected the atom of an index above 2^32 - 2.
    fn array_from_async_create_data_property(
        &mut self,
        activation: ObjectRef,
        value: Value,
    ) -> Completion<()> {
        let array = self.array_from_async_array(activation)?;
        let index = self.array_from_async_mut(activation)?.index;
        let mark = self.keep(value);
        let defined = self
            .index_key(index)
            .and_then(|key| self.create_data_property_or_throw(array, key, value));
        self.release(mark);
        defined
    }

    /// AsyncIteratorClose ( iteratorRecord, completion ) (`#sec-asynciteratorclose`) with the
    /// throw completion of `error`, as IfAbruptCloseAsyncIterator calls it: `return` read and
    /// called, and its result awaited when the call completes normally; whatever they answer,
    /// the throw of `error` once they ended (step 5), and a termination as it is.
    fn array_from_async_close(
        &mut self,
        activation: ObjectRef,
        error: Value,
    ) -> Completion<Reached> {
        let record = self.array_from_async_record(activation)?;
        self.array_from_async_mut(activation)?.error = error;
        // 3. Let innerResult be Completion(GetMethod(iterator, "return")).
        let method = self
            .name_key("return")
            .and_then(|key| self.get_method(Value::Object(record.iterator), key));
        let method = match method {
            Ok(Some(method)) => method,
            // 4.b. If return is undefined, return ? completion. 5. If completion is a throw
            // completion, return ? completion.
            Ok(None) | Err(Abrupt::Throw(_)) => return Err(Abrupt::Throw(error)),
            Err(abrupt) => return Err(abrupt),
        };
        // 4.c. Set innerResult to Completion(Call(return, iterator)).
        let result = match self.call_value(method, Value::Object(record.iterator), &[]) {
            Ok(result) => result,
            Err(Abrupt::Throw(_)) => return Err(Abrupt::Throw(error)),
            Err(abrupt) => return Err(abrupt),
        };
        // 4.d. If innerResult is a normal completion, set innerResult to
        // Completion(Await(innerResult.[[Value]])).
        match self.array_from_async_await(activation, FromAsyncStep::Closed, result) {
            Err(Abrupt::Throw(_)) => Err(Abrupt::Throw(error)),
            reached => reached,
        }
    }

    /// Step 9 of `Array.fromAsync` up to its first Await: the branch of an array-like.
    fn array_from_async_array_like(
        &mut self,
        activation: ObjectRef,
        constructor: Value,
        items: Value,
    ) -> Completion<Reached> {
        // a. NOTE: items is neither async iterable nor iterable so assume it is an array-like
        // object. b. Let arrayLike be ! ToObject(items). It may be new: the box holds it.
        let array_like = self.to_object(items)?;
        self.array_from_async_mut(activation)?.input = Input::ArrayLike {
            object: array_like,
            length: 0.0,
        };
        // c. Let length be ? LengthOfArrayLike(arrayLike).
        let length = self.length_of_array_like(array_like)?;
        self.array_from_async_mut(activation)?.input = Input::ArrayLike {
            object: array_like,
            length,
        };
        // d. If IsConstructor(ctor) is true, then i. Let array be ? Construct(ctor, « 𝔽(length)
        // »). e. Else, i. Let array be ? ArrayCreate(length).
        let array = self.array_from_async_construct(constructor, Some(length))?;
        self.array_from_async_mut(activation)?.array = Some(array);
        // f. Let k be 0. g. Repeat, while k < length,
        self.array_from_async_array_like_next(activation)
    }

    /// A turn of the loop of step 9.g, up to its Await of the element, or step 9.h and 9.i once
    /// k reaches length.
    fn array_from_async_array_like_next(&mut self, activation: ObjectRef) -> Completion<Reached> {
        let (input, index) = {
            let state = self.array_from_async_mut(activation)?;
            (state.input, state.index)
        };
        let Input::ArrayLike { object, length } = input else {
            return Err(self.fault());
        };
        if index >= length {
            // h. Perform ? Set(array, "length", 𝔽(length), true). i. Return array.
            let array = self.array_from_async_array(activation)?;
            let key = self.name_key("length")?;
            self.set(array, key, Value::Number(length), true)?;
            return Ok(Reached::Return(Value::Object(array)));
        }
        // i. Let propertyKey be ! ToString(𝔽(k)). ii. Let kValue be ? Get(arrayLike,
        // propertyKey).
        let key = self.index_key(index)?;
        let value = self.get(object, key)?;
        // iii. Set kValue to ? Await(kValue).
        self.array_from_async_await(activation, FromAsyncStep::Element, value)
    }

    /// Steps 9.g.iv to 9.g.vi with the awaited element `value`.
    fn array_from_async_element(
        &mut self,
        activation: ObjectRef,
        value: Value,
    ) -> Completion<Reached> {
        let (mapper, this_arg, index) = {
            let state = self.array_from_async_mut(activation)?;
            (state.mapper, state.this_arg, state.index)
        };
        match mapper {
            // iv. If mapping is true, then 1. Let mappedValue be ? Call(mapper, thisArg, «
            // kValue, 𝔽(k) »). 2. Set mappedValue to ? Await(mappedValue).
            Some(mapper) => {
                let mapped = self.call_value(mapper, this_arg, &[value, Value::Number(index)])?;
                self.array_from_async_await(activation, FromAsyncStep::MappedElement, mapped)
            }
            // v. Else, let mappedValue be kValue.
            None => self.array_from_async_define_element(activation, value),
        }
    }

    /// Steps 9.g.vi and 9.g.vii with the mapped value `value`, then the next turn.
    fn array_from_async_define_element(
        &mut self,
        activation: ObjectRef,
        value: Value,
    ) -> Completion<Reached> {
        // vi. Perform ? CreateDataPropertyOrThrow(array, propertyKey, mappedValue).
        self.array_from_async_create_data_property(activation, value)?;
        // vii. Set k to k + 1.
        self.array_from_async_mut(activation)?.index += 1.0;
        self.array_from_async_array_like_next(activation)
    }
}
