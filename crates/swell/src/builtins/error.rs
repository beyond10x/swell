//! Error objects (ECMA-262 20.5, `#sec-error-objects`): the `Error` constructor (20.5.1.1),
//! InstallErrorCause (20.5.8.1), `Error.prototype` with `constructor`, `message`, `name` and
//! `toString` (20.5.3), and the six NativeError constructors and prototypes of 20.5.5 and 20.5.6;
//! with the errors the engine itself throws. Written by `story:object-model-and-realms`.
//! `Error.isError` (20.5.2.1, `#sec-error.iserror`), defined on `Error` after the keys above
//! (`length`, `name`, `prototype`), is `story:global-this-uri-functions-and-is-error`.
//!
//! `%AggregateError%` and `%AggregateError.prototype%` (20.5.7, `#sec-aggregate-error-objects`)
//! are `story:promise-statics-finally-and-aggregate-error`: made by a step of their own after the
//! seven of [`ErrorKind::ALL`], since the constructor takes `errors` before `message`
//! (`#sec-aggregate-error`), with the kind `ErrorKind::AggregateError` in [[ErrorData]]
//! (`docs/design/host-interface.md`, "The kind of an AggregateError, and kinds to come").
//!
//! `Error.captureStackTrace` and `Error.prototype.stack` are not in the pinned draft.

use crate::builtins::object::argument;
use crate::heap::arena::Handle;
use crate::host::agent::Agent;
use crate::host::outcome::{Abrupt, Completion};
use crate::host::{ErrorKind, Host};
use crate::object::ObjectKind;
use crate::object::builtin_function::{Behaviour, BuiltinId};
use crate::object::ordinary::OrdinaryData;
use crate::object::property::{Descriptor, Key, Property};
use crate::realm::intrinsics::{ErrorIntrinsics, Intrinsic};
use crate::value::{ObjectRef, Value};

impl<H: Host> Agent<H> {
    /// The Error intrinsics of a realm (20.5.2, 20.5.3, 20.5.6.2, 20.5.6.3), in the order of
    /// [`ErrorKind::ALL`].
    pub(crate) fn create_error_intrinsics(
        &mut self,
        realm: Handle,
        object_prototype: ObjectRef,
        function_prototype: ObjectRef,
    ) -> Completion<[ErrorIntrinsics; 7]> {
        let mut made = Vec::new();
        let mut error: Option<ErrorIntrinsics> = None;
        for kind in ErrorKind::ALL {
            // A NativeError prototype's [[Prototype]] is %Error.prototype% (20.5.6.3) and a
            // NativeError constructor's is %Error% (20.5.6.2).
            let (prototype_parent, constructor_parent) = match error {
                None => (object_prototype, function_prototype),
                Some(error) => (error.prototype, error.constructor),
            };
            let prototype =
                self.ordinary_object_create(Some(prototype_parent), OrdinaryData::Plain)?;
            let constructor = self.create_builtin_function(
                Behaviour::Builtin(BuiltinId::ErrorConstructor(kind)),
                1,
                kind.name(),
                realm,
                Some(constructor_parent),
                true,
            )?;
            // "prototype": { [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: false }.
            let key = self.name_key("prototype")?;
            self.add_property(
                constructor,
                key,
                Property::data(Value::Object(prototype), false, false, false),
            )?;
            self.define_builtin_property(prototype, "constructor", Value::Object(constructor))?;
            let empty = self.new_string_str("")?;
            self.define_builtin_property(prototype, "message", Value::String(empty))?;
            let name = self.new_string_str(kind.name())?;
            self.define_builtin_property(prototype, "name", Value::String(name))?;
            let pair = ErrorIntrinsics {
                constructor,
                prototype,
            };
            if error.is_none() {
                let to_string = self.create_builtin_function(
                    Behaviour::Builtin(BuiltinId::ErrorPrototypeToString),
                    0,
                    "toString",
                    realm,
                    Some(function_prototype),
                    false,
                )?;
                self.define_builtin_property(prototype, "toString", Value::Object(to_string))?;
                // `Error.isError` (`#sec-error.iserror`): a built-in function of length 1, not a
                // constructor, { [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]:
                // true } (clause 18), after "prototype" on %Error%.
                let is_error = self.create_builtin_function(
                    Behaviour::Builtin(BuiltinId::ErrorIsError),
                    1,
                    "isError",
                    realm,
                    Some(function_prototype),
                    false,
                )?;
                self.define_builtin_property(constructor, "isError", Value::Object(is_error))?;
                error = Some(pair);
            }
            made.push(pair);
        }
        match <[ErrorIntrinsics; 7]>::try_from(made) {
            Ok(errors) => Ok(errors),
            Err(_) => Err(self.fault()),
        }
    }

    /// `%AggregateError%` and `%AggregateError.prototype%` of a realm
    /// (`#sec-properties-of-the-aggregate-error-constructors`,
    /// `#sec-properties-of-the-aggregate-error-prototype-objects`), made after the seven of
    /// [`ErrorKind::ALL`] from `error`, `%Error%` and `%Error.prototype%`: the constructor's
    /// [[Prototype]] is `%Error%` and its `length` 2, the prototype's [[Prototype]] is
    /// `%Error.prototype%`, with `constructor`, the empty `message` and `name`
    /// "AggregateError", in the order the NativeError prototypes have them.
    pub(crate) fn create_aggregate_error_intrinsics(
        &mut self,
        realm: Handle,
        error: ErrorIntrinsics,
    ) -> Completion<ErrorIntrinsics> {
        let kind = ErrorKind::AggregateError;
        let prototype = self.ordinary_object_create(Some(error.prototype), OrdinaryData::Plain)?;
        let constructor = self.create_builtin_function(
            Behaviour::Builtin(BuiltinId::AggregateErrorConstructor),
            2,
            kind.name(),
            realm,
            Some(error.constructor),
            true,
        )?;
        // "prototype": { [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: false }
        // (`#sec-aggregate-error.prototype`).
        let key = self.name_key("prototype")?;
        self.add_property(
            constructor,
            key,
            Property::data(Value::Object(prototype), false, false, false),
        )?;
        self.define_builtin_property(prototype, "constructor", Value::Object(constructor))?;
        let empty = self.new_string_str("")?;
        self.define_builtin_property(prototype, "message", Value::String(empty))?;
        let name = self.new_string_str(kind.name())?;
        self.define_builtin_property(prototype, "name", Value::String(name))?;
        Ok(ErrorIntrinsics {
            constructor,
            prototype,
        })
    }

    /// An error object of `kind` with the [[Prototype]] `prototype` and an own "message"
    /// `message`, as `new kind(message)` makes one: its [[ErrorData]] records `kind`.
    pub(crate) fn create_error(
        &mut self,
        kind: ErrorKind,
        prototype: ObjectRef,
        message: &str,
    ) -> Completion<ObjectRef> {
        let error = self.ordinary_object_create(Some(prototype), OrdinaryData::Error(kind))?;
        let message = self.new_string_str(message)?;
        let key = self.name_key("message")?;
        self.add_property(
            error,
            key,
            Property::data(Value::String(message), true, false, true),
        )?;
        Ok(error)
    }

    /// The throw completion of a new error of `kind` in the current realm; over the memory
    /// budget the run ends instead.
    pub(crate) fn throw_error(&mut self, kind: ErrorKind, message: &str) -> Abrupt {
        let made = self
            .current_intrinsic(Intrinsic::ErrorPrototype(kind))
            .and_then(|prototype| self.create_error(kind, prototype, message));
        match made {
            Ok(error) => Abrupt::Throw(Value::Object(error)),
            Err(abrupt) => abrupt,
        }
    }

    /// `Error ( message [ , options ] )` (`#sec-error-message`) and `NativeError ( message [ ,
    /// options ] )` (`#sec-nativeerror`).
    pub(crate) fn error_constructor(
        &mut self,
        kind: ErrorKind,
        function: ObjectRef,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        // 1. If NewTarget is undefined, let newTarget be the active function object.
        let new_target = new_target.unwrap_or(function);
        // 2. The [[ErrorData]] slot records the kind of the constructor called, whatever
        // prototype newTarget gives.
        let error = self.ordinary_create_from_constructor(
            new_target,
            Intrinsic::ErrorPrototype(kind),
            OrdinaryData::Error(kind),
        )?;
        let keep = self.keep(Value::Object(error));
        let done = self.error_constructor_steps(error, args);
        self.release(keep);
        done.map(|()| Value::Object(error))
    }

    /// Steps 3 and 4 of `Error`: the message and the cause.
    fn error_constructor_steps(&mut self, error: ObjectRef, args: &[Value]) -> Completion<()> {
        let message = args.first().copied().unwrap_or(Value::Undefined);
        if message != Value::Undefined {
            let message = self.to_string(message)?;
            let key = self.name_key("message")?;
            self.create_non_enumerable_data_property_or_throw(error, key, Value::String(message))?;
        }
        let options = args.get(1).copied().unwrap_or(Value::Undefined);
        self.install_error_cause(error, options)
    }

    /// `AggregateError ( errors, message [ , options ] )` (`#sec-aggregate-error`).
    pub(crate) fn aggregate_error_constructor(
        &mut self,
        function: ObjectRef,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        let kind = ErrorKind::AggregateError;
        // 1. If NewTarget is undefined, let newTarget be the active function object; else let
        // newTarget be NewTarget.
        let new_target = new_target.unwrap_or(function);
        // 2. Let obj be ? OrdinaryCreateFromConstructor(newTarget, "%AggregateError.prototype%",
        // « [[ErrorData]] »).
        let error = self.ordinary_create_from_constructor(
            new_target,
            Intrinsic::ErrorPrototype(kind),
            OrdinaryData::Error(kind),
        )?;
        let keep = self.keep(Value::Object(error));
        let done = self.aggregate_error_steps(error, args);
        self.release(keep);
        done.map(|()| Value::Object(error))
    }

    /// Steps 3 to 6 of `AggregateError`, with `obj` kept: the message and the cause as `Error`
    /// installs them, then the list of `errors` as an Array of the current realm in the own
    /// property "errors".
    fn aggregate_error_steps(&mut self, error: ObjectRef, args: &[Value]) -> Completion<()> {
        // 3. If message is not undefined, then ... 4. Perform ? InstallErrorCause(obj, options).
        self.error_constructor_steps(error, args.get(1..).unwrap_or(&[]))?;
        // 5. Let errorsList be ? IteratorToList(? GetIterator(errors, sync)).
        let errors = self.iterable_to_array(argument(args, 0))?;
        // 6. Perform ! DefinePropertyOrThrow(obj, "errors", PropertyDescriptor {
        // [[Configurable]]: true, [[Enumerable]]: false, [[Writable]]: true, [[Value]]:
        // CreateArrayFromList(errorsList) }).
        let mark = self.keep(Value::Object(errors));
        let defined = self.name_key("errors").and_then(|key| {
            self.define_property_or_throw(
                error,
                key,
                Descriptor::data(Value::Object(errors), true, false, true),
            )
        });
        self.release(mark);
        defined
    }

    /// CreateArrayFromList ( IteratorToList ( ? GetIterator ( iterable, sync ) ) )
    /// (`#sec-iteratortolist`, `#sec-createarrayfromlist`): the values of `iterable`, which the
    /// caller holds, appended to a new Array of the current realm as they are read; the Array
    /// grows as an Array does, charged before it grows. No script reaches it before it is
    /// answered, so it is the Array CreateArrayFromList makes of the list.
    pub(crate) fn iterable_to_array(&mut self, iterable: Value) -> Completion<ObjectRef> {
        let mut record = self.get_iterator(iterable)?;
        let mark = self.keep(Value::Object(record.iterator));
        self.keep(record.next_method);
        let answer = self.array_create(0.0, None).and_then(|array| {
            self.keep(Value::Object(array));
            self.append_iterator_values(&mut record, array)
                .map(|()| array)
        });
        self.release(mark);
        answer
    }

    /// The loop of IteratorToList: each value IteratorStepValue answers is appended to `array`,
    /// which the caller keeps with the record's [[Iterator]] and [[NextMethod]].
    fn append_iterator_values(
        &mut self,
        record: &mut crate::ops::iteration::IteratorRecord,
        array: ObjectRef,
    ) -> Completion<()> {
        let mut index: u32 = 0;
        while let Some(value) = self.iterator_step_value(record)? {
            // An Array holds at most 2^32 - 1 elements.
            if index == u32::MAX {
                return Err(self.throw_error(ErrorKind::RangeError, "Invalid array length"));
            }
            self.create_data_property_or_throw(array, Key::Index(index), value)?;
            index += 1;
        }
        Ok(())
    }

    /// An AggregateError of `realm`: [[ErrorData]] of the kind `AggregateError`, an own
    /// "message" when `message` is given, and the own "errors" `errors` { [[Writable]]: true,
    /// [[Enumerable]]: false, [[Configurable]]: true }; `errors` is kept while the error is
    /// made. Runs no code.
    pub(crate) fn create_aggregate_error(
        &mut self,
        realm: Handle,
        message: Option<&str>,
        errors: ObjectRef,
    ) -> Completion<ObjectRef> {
        let kind = ErrorKind::AggregateError;
        let mark = self.keep(Value::Object(errors));
        let made = self
            .intrinsic(realm, Intrinsic::ErrorPrototype(kind))
            .and_then(|prototype| match message {
                Some(message) => self.create_error(kind, prototype, message),
                None => self.ordinary_object_create(Some(prototype), OrdinaryData::Error(kind)),
            })
            .and_then(|error| {
                self.keep(Value::Object(error));
                let key = self.name_key("errors")?;
                self.add_property(
                    error,
                    key,
                    Property::data(Value::Object(errors), true, false, true),
                )?;
                Ok(error)
            });
        self.release(mark);
        made
    }

    /// CreateNonEnumerableDataPropertyOrThrow (`#sec-createnonenumerabledatapropertyorthrow`).
    fn create_non_enumerable_data_property_or_throw(
        &mut self,
        object: ObjectRef,
        key: crate::object::property::Key,
        value: Value,
    ) -> Completion<()> {
        self.define_property_or_throw(object, key, Descriptor::data(value, true, false, true))
    }

    /// InstallErrorCause (`#sec-installerrorcause`).
    fn install_error_cause(&mut self, error: ObjectRef, options: Value) -> Completion<()> {
        let Value::Object(options) = options else {
            return Ok(());
        };
        let key = self.name_key("cause")?;
        if self.has_property(options, key)? {
            let cause = self.get(options, key)?;
            self.create_non_enumerable_data_property_or_throw(error, key, cause)?;
        }
        Ok(())
    }

    /// `Error.isError ( arg )` (`#sec-error.iserror`). [[ErrorData]] is the data of an ordinary
    /// object of any realm, so an error of another realm answers true as well, or of a host
    /// object made with an `error_kind` (`docs/design/host-interface.md`, decision 15).
    pub(crate) fn error_is_error(&mut self, args: &[Value]) -> Completion<Value> {
        // 1. If arg is not an Object, return false.
        let Value::Object(object) = argument(args, 0) else {
            return Ok(Value::Boolean(false));
        };
        // 2. If arg does not have an [[ErrorData]] internal slot, return false. 3. Return true.
        let has_error_data = error_data(&self.engine_object(object)?.kind).is_some();
        Ok(Value::Boolean(has_error_data))
    }

    /// `Error.prototype.toString ( )` (`#sec-error.prototype.tostring`).
    pub(crate) fn error_prototype_to_string(&mut self, this: Value) -> Completion<Value> {
        let Value::Object(object) = this else {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "Error.prototype.toString called on a non-object",
            ));
        };
        let key = self.name_key("name")?;
        let name = match self.get(object, key)? {
            Value::Undefined => self.new_string_str("Error")?,
            name => self.to_string(name)?,
        };
        let keep = self.keep(Value::String(name));
        let rest = self.error_to_string_rest(object, name);
        self.release(keep);
        rest
    }

    /// Steps 5 to 9 of `Error.prototype.toString`, with `name` kept.
    fn error_to_string_rest(
        &mut self,
        object: ObjectRef,
        name: crate::value::StringRef,
    ) -> Completion<Value> {
        let key = self.name_key("message")?;
        let message = match self.get(object, key)? {
            Value::Undefined => self.new_string_str("")?,
            message => self.to_string(message)?,
        };
        let name_length = self.string_length(name)?;
        if name_length == 0 {
            return Ok(Value::String(message));
        }
        let message_length = self.string_length(message)?;
        if message_length == 0 {
            return Ok(Value::String(name));
        }
        // The length of name, ": " and msg is asked of `string_length` and charged before
        // either part is copied.
        let length = u64::try_from(name_length.saturating_add(2).saturating_add(message_length))
            .unwrap_or(u64::MAX);
        if length > self.limits.string_length {
            return Err(self.throw_error(ErrorKind::RangeError, "string_length exceeded"));
        }
        self.charge_steps(length)?;
        let mut units = self.string_units(name)?;
        units.extend([u16::from(b':'), u16::from(b' ')]);
        units.extend(self.string_units(message)?);
        Ok(Value::String(self.new_string_value(&units)?))
    }
}

impl<H: Host> Agent<H> {
    /// `cx.new_error` (`docs/design/host-interface.md`, decision 14): an error of `kind` of
    /// `realm` with the message `message`. For `ErrorKind::AggregateError` it is the object
    /// `new AggregateError([], message)` makes in `realm`, with an own "errors" that is a new
    /// empty Array of `realm` ("The kind of an AggregateError, and kinds to come"); both objects
    /// are charged and no code runs.
    pub(crate) fn new_error_in(
        &mut self,
        realm: Handle,
        kind: ErrorKind,
        message: &str,
    ) -> Completion<ObjectRef> {
        if kind == ErrorKind::AggregateError {
            let array_prototype = self.intrinsic(realm, Intrinsic::ArrayPrototype)?;
            let errors = self.array_create(0.0, Some(array_prototype))?;
            return self.create_aggregate_error(realm, Some(message), errors);
        }
        let prototype = self.intrinsic(realm, Intrinsic::ErrorPrototype(kind))?;
        self.create_error(kind, prototype, message)
    }
}

/// The kind the [[ErrorData]] slot of an object of `kind` records, or `None` when it has no such
/// slot: an ordinary object an Error or NativeError constructor made, or a host object made
/// with an `error_kind` (`docs/design/host-interface.md`, decision 15).
pub(crate) fn error_data(kind: &ObjectKind) -> Option<ErrorKind> {
    match kind {
        ObjectKind::Ordinary(ordinary) => match ordinary.data {
            OrdinaryData::Error(kind) => Some(kind),
            _ => None,
        },
        ObjectKind::Host(host) => host.error_kind,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::builtins::string::transform::tests::{Answer, Counting, FEW_STEPS, LONG};

    /// `Error.prototype.toString` (`#sec-error.prototype.tostring`), step 9, builds its result
    /// from other values, so it is bounded by `string_length` (`docs/design/host-interface.md`,
    /// section 2): with `string_length` 1,000,000, an Error whose name has 1,000,000 code
    /// units and whose message is "m" throws a `RangeError` before a unit of either is copied.
    #[test]
    fn error_to_string_past_string_length_copies_nothing() {
        let mut counting = Counting::new(LONG as u64);
        counting.string("s", &vec![0x61; LONG]);
        assert_eq!(
            counting.eval("var e = new Error(\"m\"); e.name = s; 0"),
            Answer::Normal("Number(0.0)".to_string())
        );
        assert_eq!(
            counting.counted(FEW_STEPS, "e.toString()"),
            (Answer::Throw("RangeError".to_string()), 0)
        );
    }
}
