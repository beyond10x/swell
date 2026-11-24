//! Operations on iterator objects (ECMA-262 7.4, `#sec-operations-on-iterator-objects`) for the
//! sync kind: the Iterator Record (`#sec-iterator-records`) as [`IteratorRecord`],
//! GetIteratorFromMethod (`#sec-getiteratorfrommethod`) with GetIteratorDirect
//! (`#sec-getiteratordirect`), GetIterator (`#sec-getiterator`), IteratorStepValue
//! (`#sec-iteratorstepvalue`) with IteratorStep (`#sec-iteratorstep`), IteratorNext
//! (`#sec-iteratornext`), IteratorComplete (`#sec-iteratorcomplete`) and IteratorValue
//! (`#sec-iteratorvalue`), IteratorClose (`#sec-iteratorclose`) and CreateIteratorResultObject
//! (`#sec-createiterresultobject`). Written by `story:iterator-protocol-and-array-string-iterators`.
//! GetIterator of the async kind, with CreateAsyncFromSyncIterator for a sync iterable, is
//! [`Agent::get_async_iterator`], written by `story:for-await-and-async-from-sync-iterator`.
//!
//! The design is `docs/design/engine.md`, decision 3, "Iterators held by a builtin":
//!
//! - A record holds handles only. Its holder keeps [[Iterator]] and [[NextMethod]] on the
//!   agent's kept stack ([`Agent::keep`]) from GetIterator until it lets the record go, on every
//!   way out ([`Agent::release`] to its mark). An operation keeps, for as long as it holds it
//!   across a call that may run code, every other value it holds: the iterator while "next" is
//!   read, the result of `next` while "done" and "value" are read, and the completion that
//!   IteratorClose is given while `return` runs. An operation that answers a value runs no code
//!   after it made it, so its caller keeps the value before its next safe point.
//! - Every call of `next` and of `return` goes through the agent's call path ([`Agent::call_value`]),
//!   which charges the call's step and counts it against the depth bounds; these operations
//!   charge no step of their own (item 2).
//! - IteratorClose is given a normal or a throw completion; a termination passes up untouched,
//!   with no call of `return`, since no code runs after it (item 4).
//!
//! The readers of an iterator that call these operations today are the instructions of the
//! `for`-`of` statement (`interp/for_of.rs`, `story:for-of-statement`) and the builtins
//! `Array.from` (`builtins/array/from.rs`), `Object.fromEntries` and `Object.groupBy`
//! (`builtins/object/from_entries.rs`, `story:array-from-and-object-from-entries`). Array
//! patterns and spread come with a later story of slice 6
//! (`story:array-destructuring-and-spread`).

use crate::host::agent::Agent;
use crate::host::outcome::{Abrupt, Completion};
use crate::host::{ErrorKind, Host};
use crate::object::ordinary::OrdinaryData;
use crate::object::property::{Key, Property};
use crate::realm::intrinsics::Intrinsic;
use crate::value::{ObjectRef, Value};

/// An Iterator Record (`#sec-iterator-records`, Table 13) of the sync kind.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct IteratorRecord {
    /// [[Iterator]]: an object that conforms to the iterator interface.
    pub(crate) iterator: ObjectRef,
    /// [[NextMethod]]: the `next` method of [[Iterator]], read once; any value.
    pub(crate) next_method: Value,
    /// [[Done]]: whether the iterator has completed or been closed.
    pub(crate) done: bool,
}

impl<H: Host> Agent<H> {
    /// GetIteratorFromMethod ( obj, method ) (`#sec-getiteratorfrommethod`). The caller holds
    /// `object` and `method` across the call; the record answered is not kept.
    pub(crate) fn get_iterator_from_method(
        &mut self,
        object: Value,
        method: Value,
    ) -> Completion<IteratorRecord> {
        // 1. Let iterator be ? Call(method, obj).
        let iterator = self.call_value(method, object, &[])?;
        // 2. If iterator is not an Object, throw a TypeError exception.
        let Value::Object(iterator) = iterator else {
            return Err(self.throw_error(ErrorKind::TypeError, "the iterator is not an object"));
        };
        // 3. Return ? GetIteratorDirect(iterator).
        self.get_iterator_direct(iterator)
    }

    /// GetIteratorDirect ( obj ) (`#sec-getiteratordirect`): `obj` is kept while "next" is read,
    /// a Get that may run a getter.
    fn get_iterator_direct(&mut self, iterator: ObjectRef) -> Completion<IteratorRecord> {
        let mark = self.keep(Value::Object(iterator));
        // 1. Let nextMethod be ? Get(obj, "next").
        let next_method = self
            .name_key("next")
            .and_then(|key| self.get(iterator, key));
        self.release(mark);
        // 2. Let iteratorRecord be the Iterator Record { [[Iterator]]: obj, [[NextMethod]]:
        // nextMethod, [[Done]]: false }. 3. Return iteratorRecord.
        Ok(IteratorRecord {
            iterator,
            next_method: next_method?,
            done: false,
        })
    }

    /// GetIterator ( obj, sync ) (`#sec-getiterator`). The caller holds `object` across the
    /// call; the record answered is not kept.
    pub(crate) fn get_iterator(&mut self, object: Value) -> Completion<IteratorRecord> {
        let Some(symbols) = self.well_known() else {
            return Err(self.fault());
        };
        // 2.a. Let method be ? GetMethod(obj, %Symbol.iterator%).
        let method = self.get_method(object, Key::Symbol(symbols.iterator))?;
        // 2.b. If method is undefined, throw a TypeError exception.
        let Some(method) = method else {
            return Err(self.throw_error(ErrorKind::TypeError, "the value is not iterable"));
        };
        // 2.c. Return ? GetIteratorFromMethod(obj, method). Nothing ran since GetMethod read
        // method, and the call holds it.
        self.get_iterator_from_method(object, method)
    }

    /// GetIterator ( obj, async ) (`#sec-getiterator`): the method of `%Symbol.asyncIterator%`,
    /// or CreateAsyncFromSyncIterator (`builtins/async_iteration.rs`) of the Iterator Record of
    /// the method of `%Symbol.iterator%`. The caller holds `object` across the call; the record
    /// answered is not kept.
    pub(crate) fn get_async_iterator(&mut self, object: Value) -> Completion<IteratorRecord> {
        let Some(symbols) = self.well_known() else {
            return Err(self.fault());
        };
        // 1.a. Let method be ? GetMethod(obj, %Symbol.asyncIterator%).
        let method = self.get_method(object, Key::Symbol(symbols.async_iterator))?;
        if let Some(method) = method {
            // 4. Return ? GetIteratorFromMethod(obj, method).
            return self.get_iterator_from_method(object, method);
        }
        // 1.b.i. Let syncMethod be ? GetMethod(obj, %Symbol.iterator%).
        let sync_method = self.get_method(object, Key::Symbol(symbols.iterator))?;
        // 1.b.ii. If syncMethod is undefined, throw a TypeError exception.
        let Some(sync_method) = sync_method else {
            return Err(self.throw_error(ErrorKind::TypeError, "the value is not async iterable"));
        };
        // 1.b.iii. Let syncIteratorRecord be ? GetIteratorFromMethod(obj, syncMethod).
        let sync = self.get_iterator_from_method(object, sync_method)?;
        // 1.b.iv. Return CreateAsyncFromSyncIterator(syncIteratorRecord).
        self.create_async_from_sync_iterator(sync)
    }

    /// IteratorNext ( iteratorRecord ) (`#sec-iteratornext`) with no value: the call of
    /// [[NextMethod]] with [[Iterator]] as the this value holds both.
    fn iterator_next(&mut self, record: &mut IteratorRecord) -> Completion<ObjectRef> {
        self.iterator_next_with(record, None)
    }

    /// IteratorNext ( iteratorRecord [ , value ] ) (`#sec-iteratornext`), with `value` when it
    /// is present (`%AsyncFromSyncIteratorPrototype%.next`, `builtins/async_iteration.rs`).
    pub(crate) fn iterator_next_with(
        &mut self,
        record: &mut IteratorRecord,
        value: Option<Value>,
    ) -> Completion<ObjectRef> {
        // 1.a. Let result be Completion(Call(iteratorRecord.[[NextMethod]],
        // iteratorRecord.[[Iterator]])). 2.a. Else, Let result be
        // Completion(Call(iteratorRecord.[[NextMethod]], iteratorRecord.[[Iterator]], « value »)).
        let this = Value::Object(record.iterator);
        let result = match value {
            Some(value) => self.call_value(record.next_method, this, &[value]),
            None => self.call_value(record.next_method, this, &[]),
        };
        match result {
            // 4. Set result to ! result.
            Ok(Value::Object(result)) => Ok(result),
            // 5. If result is not an Object, then set iteratorRecord.[[Done]] to true and throw
            // a TypeError exception.
            Ok(_) => {
                record.done = true;
                Err(self.throw_error(ErrorKind::TypeError, "the iterator result is not an object"))
            }
            // 3. If result is a throw completion, then set iteratorRecord.[[Done]] to true and
            // return ? result. A termination ends the run, whatever [[Done]] is.
            Err(abrupt) => {
                record.done = true;
                Err(abrupt)
            }
        }
    }

    /// IteratorComplete ( iteratorResult ) (`#sec-iteratorcomplete`); the test of `done` of the
    /// loop of `yield*` (`interp/yield_star.rs`) calls it too.
    pub(crate) fn iterator_complete(&mut self, result: ObjectRef) -> Completion<bool> {
        // 1. Return ToBoolean(? Get(iteratorResult, "done")).
        let key = self.name_key("done")?;
        let done = self.get(result, key)?;
        self.to_boolean(done)
    }

    /// IteratorValue ( iteratorResult ) (`#sec-iteratorvalue`).
    pub(crate) fn iterator_value(&mut self, result: ObjectRef) -> Completion<Value> {
        // 1. Return ? Get(iteratorResult, "value").
        let key = self.name_key("value")?;
        self.get(result, key)
    }

    /// IteratorStep ( iteratorRecord ) (`#sec-iteratorstep`): `None` for done. The result is
    /// kept while "done" is read.
    fn iterator_step(&mut self, record: &mut IteratorRecord) -> Completion<Option<ObjectRef>> {
        // 1. Let result be ? IteratorNext(iteratorRecord).
        let result = self.iterator_next(record)?;
        // 2. Let done be Completion(IteratorComplete(result)).
        let mark = self.keep(Value::Object(result));
        let done = self.iterator_complete(result);
        self.release(mark);
        match done {
            // 3. If done is a throw completion, then set iteratorRecord.[[Done]] to true and
            // return ? done.
            Err(abrupt) => {
                record.done = true;
                Err(abrupt)
            }
            // 5. If done is true, then set iteratorRecord.[[Done]] to true and return done.
            Ok(true) => {
                record.done = true;
                Ok(None)
            }
            // 6. Return result.
            Ok(false) => Ok(Some(result)),
        }
    }

    /// IteratorStepValue ( iteratorRecord ) (`#sec-iteratorstepvalue`): `None` for done. The
    /// caller keeps the record's [[Iterator]] and [[NextMethod]]; the result is kept while
    /// "value" is read.
    pub(crate) fn iterator_step_value(
        &mut self,
        record: &mut IteratorRecord,
    ) -> Completion<Option<Value>> {
        // 1. Let result be ? IteratorStep(iteratorRecord). 2. If result is done, then return
        // done.
        let Some(result) = self.iterator_step(record)? else {
            return Ok(None);
        };
        // 3. Let value be Completion(IteratorValue(result)).
        let mark = self.keep(Value::Object(result));
        let value = self.iterator_value(result);
        self.release(mark);
        // 4. If value is a throw completion, then set iteratorRecord.[[Done]] to true. 5.
        // Return ? value.
        if value.is_err() {
            record.done = true;
        }
        value.map(Some)
    }

    /// IteratorClose ( iteratorRecord, completion ) (`#sec-iteratorclose`). `completion` is a
    /// normal or a throw completion; a termination is answered as it is, and `return` is not
    /// called (`docs/design/engine.md`, decision 3, item 4). The caller keeps the record's
    /// [[Iterator]]; the value of `completion` is kept while `return` is read and called.
    pub(crate) fn iterator_close(
        &mut self,
        record: &IteratorRecord,
        completion: Completion<Value>,
    ) -> Completion<Value> {
        let held = match completion {
            Ok(value) | Err(Abrupt::Throw(value)) => value,
            Err(Abrupt::Terminated(_)) => return completion,
        };
        let mark = self.keep(held);
        let closed = self.iterator_close_steps(record.iterator, completion);
        self.release(mark);
        closed
    }

    /// Steps 2 to 8 of IteratorClose, with the completion's value kept.
    fn iterator_close_steps(
        &mut self,
        iterator: ObjectRef,
        completion: Completion<Value>,
    ) -> Completion<Value> {
        // 2. Let iterator be iteratorRecord.[[Iterator]]. 3. Let innerResult be
        // Completion(GetMethod(iterator, "return")).
        let inner = self
            .name_key("return")
            .and_then(|key| self.get_method(Value::Object(iterator), key));
        // 4. If innerResult is a normal completion, then
        let inner = match inner {
            // 4.b. If return is undefined, return ? completion.
            Ok(None) => return completion,
            // 4.c. Set innerResult to Completion(Call(return, iterator)).
            Ok(Some(method)) => self.call_value(method, Value::Object(iterator), &[]),
            Err(abrupt) => Err(abrupt),
        };
        // A termination of GetMethod or of `return` ends the run, whatever completion is.
        if let Err(Abrupt::Terminated(_)) = inner {
            return inner;
        }
        // 5. If completion is a throw completion, return ? completion.
        completion?;
        // 6. If innerResult is a throw completion, return ? innerResult.
        // 7. If innerResult.[[Value]] is not an Object, throw a TypeError exception.
        if !matches!(inner?, Value::Object(_)) {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "the result of the iterator's return is not an object",
            ));
        }
        // 8. Return ? completion.
        completion
    }

    /// CreateIteratorResultObject ( value, done ) (`#sec-createiterresultobject`): an ordinary
    /// object of the current realm with "value" then "done". The object is new and no code runs,
    /// so each CreateDataPropertyOrThrow adds the property; `value` is held across no safe point.
    pub(crate) fn create_iter_result_object(
        &mut self,
        value: Value,
        done: bool,
    ) -> Completion<ObjectRef> {
        // 1. Let obj be OrdinaryObjectCreate(%Object.prototype%).
        let prototype = self.current_intrinsic(Intrinsic::ObjectPrototype)?;
        let object = self.ordinary_object_create(Some(prototype), OrdinaryData::Plain)?;
        // 2. Perform ! CreateDataPropertyOrThrow(obj, "value", value).
        let key = self.name_key("value")?;
        self.add_property(object, key, Property::data(value, true, true, true))?;
        // 3. Perform ! CreateDataPropertyOrThrow(obj, "done", done).
        let key = self.name_key("done")?;
        self.add_property(
            object,
            key,
            Property::data(Value::Boolean(done), true, true, true),
        )?;
        // 4. Return obj.
        Ok(object)
    }
}

#[cfg(test)]
mod tests {
    use super::IteratorRecord;
    use crate::host::ErrorKind;
    use crate::host::agent::Agent;
    use crate::host::hooks::test_host::{NoHooks, agent_with_realm, enter};
    use crate::host::outcome::{Abrupt, Completion};
    use crate::realm::intrinsics::Intrinsic;
    use crate::value::{ObjectRef, Root, Value};
    use crate::{Outcome, Parsed, Realm, Source, Termination};

    /// An agent whose script `source` answered an object, with the root that holds the object
    /// and its realm entered, so that a test runs the operations on the agent directly.
    struct Prepared {
        agent: Agent<NoHooks>,
        _realm: Realm,
        root: Root,
    }

    impl Prepared {
        fn new(source: &str) -> Prepared {
            let (mut agent, realm) = agent_with_realm();
            let Ok(Parsed::Parsed(script)) = agent.parse_script(&realm, Source::Utf8(source), None)
            else {
                panic!("{source} did not parse");
            };
            let outcome = agent.evaluate_script(&script);
            let Ok(Outcome::Normal(root)) = outcome else {
                panic!("{source}: {outcome:?}");
            };
            enter(&mut agent, &realm);
            Prepared {
                agent,
                _realm: realm,
                root,
            }
        }

        /// The object the script answered.
        fn state(&mut self) -> ObjectRef {
            match self.agent.heap.roots.get(&self.root) {
                Ok(Value::Object(object)) => object,
                other => panic!("the script answered {other:?}"),
            }
        }

        /// The property `name` of the object the script answered.
        fn read(&mut self, name: &str) -> Value {
            let state = self.state();
            let key = self.agent.name_key(name).unwrap();
            self.agent.get(state, key).unwrap()
        }

        /// GetIterator of the property `name`, with [[Iterator]] and [[NextMethod]] kept.
        fn iterator_of(&mut self, name: &str) -> IteratorRecord {
            let iterable = self.read(name);
            let record = self.agent.get_iterator(iterable).unwrap();
            self.agent.keep(Value::Object(record.iterator));
            self.agent.keep(record.next_method);
            record
        }

        /// Whether `value` is a `TypeError` of the realm.
        fn is_type_error(&mut self, value: Value) -> bool {
            let Value::Object(error) = value else {
                return false;
            };
            let prototype = self
                .agent
                .current_intrinsic(Intrinsic::ErrorPrototype(ErrorKind::TypeError))
                .unwrap();
            self.agent.get_prototype_of(error) == Ok(Some(prototype))
        }

        /// Whether `completion` is a throw of a `TypeError` of the realm.
        fn throws_type_error<T: std::fmt::Debug>(&mut self, completion: Completion<T>) -> bool {
            match completion {
                Err(Abrupt::Throw(thrown)) => self.is_type_error(thrown),
                other => panic!("expected a throw, got {other:?}"),
            }
        }
    }

    /// An iterable whose iterator counts the reads of its "next" in `nextReads` and answers 10,
    /// 20 and 30, then done.
    const COUNTING: &str = "var state = { nextReads: 0 }, counting = {};\n\
         counting[Symbol.iterator] = function () {\n\
           var i = 0, it = {};\n\
           Object.defineProperty(it, \"next\", { get: function () {\n\
             state.nextReads++;\n\
             return function () { i++; return { value: i * 10, done: i > 3 }; };\n\
           } });\n\
           return it;\n\
         };\n\
         state.counting = counting;\n";

    /// GetIterator (`#sec-getiterator`) with GetIteratorFromMethod and GetIteratorDirect reads
    /// "next" once; IteratorStepValue (`#sec-iteratorstepvalue`) answers each value and then
    /// done, which sets [[Done]]. GetIterator of a String steps its code points through
    /// `String.prototype[Symbol.iterator]`, and the iterator of an Array its elements.
    #[test]
    fn get_iterator_reads_next_once_and_step_value_steps_to_done() {
        let mut prepared = Prepared::new(&format!(
            "{COUNTING}state.text = \"ab\"; state.list = [1, 2]; state"
        ));
        let mut record = prepared.iterator_of("counting");
        let mut values = Vec::new();
        while let Some(value) = prepared.agent.iterator_step_value(&mut record).unwrap() {
            assert!(!record.done);
            values.push(value);
        }
        assert!(record.done);
        assert_eq!(
            values,
            [10.0, 20.0, 30.0].map(Value::Number).to_vec(),
            "the values"
        );
        assert_eq!(prepared.read("nextReads"), Value::Number(1.0));

        let mut record = prepared.iterator_of("text");
        let mut units = Vec::new();
        while let Some(value) = prepared.agent.iterator_step_value(&mut record).unwrap() {
            let Value::String(string) = value else {
                panic!("{value:?}");
            };
            units.extend(prepared.agent.string_units_for_tests(string).unwrap());
        }
        assert_eq!(String::from_utf16_lossy(&units), "ab");
        let mut record = prepared.iterator_of("list");
        assert_eq!(
            prepared.agent.iterator_step_value(&mut record),
            Ok(Some(Value::Number(1.0)))
        );
        assert_eq!(
            prepared.agent.iterator_step_value(&mut record),
            Ok(Some(Value::Number(2.0)))
        );
        assert_eq!(prepared.agent.iterator_step_value(&mut record), Ok(None));
        assert!(record.done);
    }

    /// GetIterator (`#sec-getiterator`, step 2.b) throws a `TypeError` for a value without
    /// `[Symbol.iterator]`, GetMethod (`#sec-getmethod`) for one whose `[Symbol.iterator]` is not
    /// callable, and GetIteratorFromMethod (step 2) for an iterator that is not an object;
    /// a throw of `[Symbol.iterator]` is answered as it is.
    #[test]
    fn get_iterator_throws_for_a_value_that_is_not_iterable() {
        let mut prepared = Prepared::new(
            "var state = { plain: {}, notCallable: {}, primitive: {}, throwing: {}, marker: {} };\n\
             state.notCallable[Symbol.iterator] = 1;\n\
             state.primitive[Symbol.iterator] = function () { return 1; };\n\
             state.throwing[Symbol.iterator] = function () { throw state.marker; };\n\
             state",
        );
        for name in ["plain", "notCallable", "primitive"] {
            let iterable = prepared.read(name);
            let got = prepared.agent.get_iterator(iterable);
            assert!(prepared.throws_type_error(got), "{name}");
        }
        let iterable = prepared.read("throwing");
        let marker = prepared.read("marker");
        assert_eq!(
            prepared.agent.get_iterator(iterable),
            Err(Abrupt::Throw(marker))
        );
        let got = prepared.agent.get_iterator(Value::Undefined);
        assert!(prepared.throws_type_error(got), "undefined");
    }

    /// IteratorStepValue (`#sec-iteratorstepvalue`) with IteratorStep, IteratorNext,
    /// IteratorComplete and IteratorValue: a throw of `next`, a result of `next` that is not an
    /// object (a `TypeError`), a [[NextMethod]] that is not callable (the `TypeError` of Call),
    /// a throw of reading "done" and a throw of reading "value" each end the step with that
    /// throw and set [[Done]] to true.
    #[test]
    fn iterator_step_value_sets_done_on_each_throw() {
        let mut prepared = Prepared::new(
            "var state = { marker: {} };\n\
             function iterable(next) { var o = {}; o[Symbol.iterator] = function () { return { next: next }; }; return o; }\n\
             state.nextThrows = iterable(function () { throw state.marker; });\n\
             state.notObject = iterable(function () { return 1; });\n\
             state.notCallable = iterable(1);\n\
             state.doneThrows = iterable(function () { return { get done() { throw state.marker; } }; });\n\
             state.valueThrows = iterable(function () { return { done: false, get value() { throw state.marker; } }; });\n\
             state",
        );
        let marker = prepared.read("marker");
        for (name, type_error) in [
            ("nextThrows", false),
            ("notObject", true),
            ("notCallable", true),
            ("doneThrows", false),
            ("valueThrows", false),
        ] {
            let mut record = prepared.iterator_of(name);
            let stepped = prepared.agent.iterator_step_value(&mut record);
            assert!(record.done, "{name}");
            if type_error {
                assert!(prepared.throws_type_error(stepped), "{name}");
            } else {
                assert_eq!(stepped, Err(Abrupt::Throw(marker)), "{name}");
            }
        }
    }

    /// Iterators whose `return` records each call in `state.calls` with its this value in
    /// `state.self`, and answers or throws as its name says.
    const CLOSING: &str = "var state = { calls: 0, marker: {}, given: {}, answer: {} };\n\
         function iterable(ret) {\n\
           var o = {};\n\
           o[Symbol.iterator] = function () {\n\
             var it = { next: function () { return { done: false }; } };\n\
             if (ret !== undefined) it[\"return\"] = ret;\n\
             return it;\n\
           };\n\
           return o;\n\
         }\n\
         function counted(answer) { return function () { state.calls++; state.self = this; return answer(); }; }\n\
         state.none = iterable(undefined);\n\
         state.object = iterable(counted(function () { return state.answer; }));\n\
         state.throwing = iterable(counted(function () { throw state.marker; }));\n\
         state.primitive = iterable(counted(function () { return 1; }));\n\
         state.notCallable = iterable(5);\n\
         state";

    /// IteratorClose (`#sec-iteratorclose`) with a normal and a throw completion. The given
    /// completion is answered when there is no `return`, and when `return` answers an object,
    /// after one call with the iterator as the this value. With a normal completion a throw of
    /// `return` is answered, and a `return` that answers a value that is not an object, or is
    /// not callable, throws a `TypeError`; with a throw completion each of them answers that
    /// throw, `return` called. A termination is answered as it is, and `return` is not called
    /// (`docs/design/engine.md`, decision 3, item 4).
    #[test]
    fn iterator_close_answers_by_its_completion_and_return() {
        let mut prepared = Prepared::new(CLOSING);
        let given = prepared.read("given");
        let marker = prepared.read("marker");
        let answer = Value::Number(7.0);
        let throw = Err(Abrupt::Throw(given));

        let record = prepared.iterator_of("none");
        assert_eq!(
            prepared.agent.iterator_close(&record, Ok(answer)),
            Ok(answer)
        );
        assert_eq!(prepared.agent.iterator_close(&record, throw), throw);

        let record = prepared.iterator_of("object");
        assert_eq!(
            prepared.agent.iterator_close(&record, Ok(answer)),
            Ok(answer)
        );
        assert_eq!(prepared.read("calls"), Value::Number(1.0));
        assert_eq!(prepared.read("self"), Value::Object(record.iterator));
        assert_eq!(prepared.agent.iterator_close(&record, throw), throw);
        assert_eq!(prepared.read("calls"), Value::Number(2.0));

        let record = prepared.iterator_of("throwing");
        assert_eq!(
            prepared.agent.iterator_close(&record, Ok(answer)),
            Err(Abrupt::Throw(marker))
        );
        assert_eq!(prepared.agent.iterator_close(&record, throw), throw);
        assert_eq!(prepared.read("calls"), Value::Number(4.0));

        let record = prepared.iterator_of("primitive");
        let closed = prepared.agent.iterator_close(&record, Ok(answer));
        assert!(prepared.throws_type_error(closed), "primitive");
        assert_eq!(prepared.agent.iterator_close(&record, throw), throw);
        assert_eq!(prepared.read("calls"), Value::Number(6.0));

        let record = prepared.iterator_of("notCallable");
        let closed = prepared.agent.iterator_close(&record, Ok(answer));
        assert!(prepared.throws_type_error(closed), "notCallable");
        assert_eq!(prepared.agent.iterator_close(&record, throw), throw);

        let record = prepared.iterator_of("object");
        let terminated = Err(Abrupt::Terminated(Termination::StepBudget));
        assert_eq!(
            prepared.agent.iterator_close(&record, terminated),
            terminated
        );
        assert_eq!(prepared.read("calls"), Value::Number(6.0));
    }

    /// CreateIteratorResultObject (`#sec-createiterresultobject`): an ordinary object whose
    /// [[Prototype]] is `%Object.prototype%`, with the own properties "value" and "done" in that
    /// order, each writable, enumerable and configurable.
    #[test]
    fn create_iter_result_object_has_value_then_done() {
        let mut prepared = Prepared::new("({})");
        let object = prepared
            .agent
            .create_iter_result_object(Value::Number(3.0), true)
            .unwrap();
        prepared.agent.keep(Value::Object(object));
        let object_prototype = prepared
            .agent
            .current_intrinsic(Intrinsic::ObjectPrototype)
            .unwrap();
        assert_eq!(
            prepared.agent.get_prototype_of(object),
            Ok(Some(object_prototype))
        );
        let keys = prepared.agent.own_property_keys(object).unwrap();
        let expected = [
            prepared.agent.name_key("value").unwrap(),
            prepared.agent.name_key("done").unwrap(),
        ];
        assert_eq!(keys.keys, expected.to_vec());
        prepared.agent.drop_key_list(keys);
        for (key, value) in expected
            .into_iter()
            .zip([Value::Number(3.0), Value::Boolean(true)])
        {
            assert_eq!(
                prepared.agent.get_own_property(object, key),
                Ok(Some(crate::object::property::Property::data(
                    value, true, true, true
                )))
            );
        }
    }
}
