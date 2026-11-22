//! Error objects (ECMA-262 20.5, `#sec-error-objects`): the `Error` constructor (20.5.1.1),
//! InstallErrorCause (20.5.8.1), `Error.prototype` with `constructor`, `message`, `name` and
//! `toString` (20.5.3), and the six NativeError constructors and prototypes of 20.5.5 and 20.5.6;
//! with the errors the engine itself throws. Written by `story:object-model-and-realms`.
//! `Error.isError` (20.5.2.1, `#sec-error.iserror`), defined on `Error` after the keys above
//! (`length`, `name`, `prototype`), is `story:global-this-uri-functions-and-is-error`.
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
use crate::object::property::{Descriptor, Property};
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

    /// An error object with the [[Prototype]] `prototype` and an own "message" `message`, as
    /// `new Error(message)` makes one.
    pub(crate) fn create_error(
        &mut self,
        prototype: ObjectRef,
        message: &str,
    ) -> Completion<ObjectRef> {
        let error = self.ordinary_object_create(Some(prototype), OrdinaryData::Error)?;
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
            .and_then(|prototype| self.create_error(prototype, message));
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
        let error = self.ordinary_create_from_constructor(
            new_target,
            Intrinsic::ErrorPrototype(kind),
            OrdinaryData::Error,
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
    /// object of any realm, so an error of another realm answers true as well.
    pub(crate) fn error_is_error(&mut self, args: &[Value]) -> Completion<Value> {
        // 1. If arg is not an Object, return false.
        let Value::Object(object) = argument(args, 0) else {
            return Ok(Value::Boolean(false));
        };
        // 2. If arg does not have an [[ErrorData]] internal slot, return false. 3. Return true.
        let has_error_data = matches!(
            &self.engine_object(object)?.kind,
            ObjectKind::Ordinary(ordinary) if matches!(ordinary.data, OrdinaryData::Error)
        );
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
    /// `realm` with the message `message`.
    pub(crate) fn new_error_in(
        &mut self,
        realm: Handle,
        kind: ErrorKind,
        message: &str,
    ) -> Completion<ObjectRef> {
        let prototype = self.intrinsic(realm, Intrinsic::ErrorPrototype(kind))?;
        self.create_error(prototype, message)
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
