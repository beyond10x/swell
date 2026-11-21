//! The intrinsics of a realm (ECMA-262 9.3.2, `#sec-createintrinsics`; 6.1.7.4,
//! `#sec-well-known-intrinsic-objects`) that this part has: `%Object.prototype%`,
//! `%Function.prototype%`, `%ThrowTypeError%`, `%Array.prototype%`, `%String.prototype%`,
//! `%Boolean.prototype%`, `%Number.prototype%`, `%Error%`, `%Error.prototype%` and the six
//! NativeErrors with their prototypes. Their methods and the other constructors are
//! `story:harness-builtins` and later slices. Written by `story:object-model-and-realms`.
//!
//! No step is charged and no safe point is passed while they are made, so the objects made
//! first need no root until the realm record holds them all.

use crate::heap::arena::Handle;
use crate::heap::collect::Mark;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};
use crate::object::builtin_function::{Behaviour, BuiltinId};
use crate::object::ordinary::OrdinaryData;
use crate::object::property::{Property, Slot};
use crate::value::{ObjectRef, Value};

/// An intrinsic of this part, by its name in Table 6.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Intrinsic {
    ObjectPrototype,
    FunctionPrototype,
    ArrayPrototype,
    StringPrototype,
    BooleanPrototype,
    NumberPrototype,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the restricted properties of functions: story:interpreter-and-script-entry-points"
        )
    )]
    ThrowTypeError,
    /// `%Error%` and the NativeError constructors.
    Error(ErrorKind),
    /// `%Error.prototype%` and the NativeError prototypes.
    ErrorPrototype(ErrorKind),
}

/// One Error constructor and its prototype.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ErrorIntrinsics {
    pub(crate) constructor: ObjectRef,
    pub(crate) prototype: ObjectRef,
}

/// [[Intrinsics]] of a realm.
pub(crate) struct Intrinsics {
    object_prototype: ObjectRef,
    function_prototype: ObjectRef,
    array_prototype: ObjectRef,
    string_prototype: ObjectRef,
    boolean_prototype: ObjectRef,
    number_prototype: ObjectRef,
    throw_type_error: ObjectRef,
    /// In the order of [`ErrorKind::ALL`].
    errors: [ErrorIntrinsics; 7],
}

impl Intrinsics {
    /// The intrinsic `which`.
    pub(crate) fn get(&self, which: Intrinsic) -> ObjectRef {
        match which {
            Intrinsic::ObjectPrototype => self.object_prototype,
            Intrinsic::FunctionPrototype => self.function_prototype,
            Intrinsic::ArrayPrototype => self.array_prototype,
            Intrinsic::StringPrototype => self.string_prototype,
            Intrinsic::BooleanPrototype => self.boolean_prototype,
            Intrinsic::NumberPrototype => self.number_prototype,
            Intrinsic::ThrowTypeError => self.throw_type_error,
            Intrinsic::Error(kind) => self.error(kind).constructor,
            Intrinsic::ErrorPrototype(kind) => self.error(kind).prototype,
        }
    }

    fn error(&self, kind: ErrorKind) -> ErrorIntrinsics {
        let [error, eval, range, reference, syntax, type_error, uri] = self.errors;
        match kind {
            ErrorKind::Error => error,
            ErrorKind::EvalError => eval,
            ErrorKind::RangeError => range,
            ErrorKind::ReferenceError => reference,
            ErrorKind::SyntaxError => syntax,
            ErrorKind::TypeError => type_error,
            ErrorKind::UriError => uri,
        }
    }

    /// Every intrinsic.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        for object in [
            self.object_prototype,
            self.function_prototype,
            self.array_prototype,
            self.string_prototype,
            self.boolean_prototype,
            self.number_prototype,
            self.throw_type_error,
        ] {
            mark.object(object);
        }
        for error in &self.errors {
            mark.object(error.constructor);
            mark.object(error.prototype);
        }
    }
}

impl<H: Host> Agent<H> {
    /// Define a property `object` does not have yet, with the attributes of a builtin's
    /// property (clause 18: { [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]: true }).
    pub(crate) fn define_builtin_property(
        &mut self,
        object: ObjectRef,
        name: &str,
        value: Value,
    ) -> Completion<()> {
        let key = self.name_key(name)?;
        self.add_property(object, key, Property::data(value, true, false, true))
    }

    /// CreateIntrinsics (`#sec-createintrinsics`) for the intrinsics of this part.
    pub(crate) fn create_intrinsics(&mut self, realm: Handle) -> Completion<()> {
        // %Object.prototype% (20.1.3): an immutable prototype exotic object whose
        // [[Prototype]] is null.
        let object_prototype =
            self.ordinary_object_create(None, OrdinaryData::ImmutablePrototype)?;
        // %Function.prototype% (20.2.3): a built-in function object that "accepts any
        // arguments and returns undefined", with "length" 0 and "name" "".
        let function_prototype = self.create_builtin_function(
            Behaviour::Builtin(BuiltinId::FunctionPrototype),
            0,
            "",
            realm,
            Some(object_prototype),
            false,
        )?;
        let throw_type_error = self.create_throw_type_error(realm, function_prototype)?;
        self.add_restricted_function_properties(function_prototype, throw_type_error)?;
        // %Array.prototype% (23.1.3): "an Array exotic object".
        let array_prototype = self.array_create(0.0, Some(object_prototype))?;
        // %String.prototype% (22.1.3): "a String exotic object" whose [[StringData]] is the
        // empty String.
        let empty = self.new_string_str("")?;
        let string_prototype = self.string_create(empty, object_prototype)?;
        let boolean_prototype = self.create_boolean_prototype(object_prototype)?;
        let number_prototype = self.create_number_prototype(object_prototype)?;
        let errors = self.create_error_intrinsics(realm, object_prototype, function_prototype)?;
        let intrinsics = Intrinsics {
            object_prototype,
            function_prototype,
            array_prototype,
            string_prototype,
            boolean_prototype,
            number_prototype,
            throw_type_error,
            errors,
        };
        match self.heap.realms.get_mut(realm) {
            Ok(record) => {
                record.intrinsics = Some(intrinsics);
                Ok(())
            }
            Err(_) => Err(self.fault()),
        }
    }

    /// `%ThrowTypeError%` (`#sec-%throwtypeerror%`): "length" and "name" are { [[Writable]]:
    /// false, [[Enumerable]]: false, [[Configurable]]: false } and [[Extensible]] is false.
    fn create_throw_type_error(
        &mut self,
        realm: Handle,
        function_prototype: ObjectRef,
    ) -> Completion<ObjectRef> {
        let thrower = self.create_builtin_function(
            Behaviour::Builtin(BuiltinId::ThrowTypeError),
            0,
            "",
            realm,
            Some(function_prototype),
            false,
        )?;
        let keys = [self.name_key("length")?, self.name_key("name")?];
        let record = self.engine_object_mut(thrower)?;
        for key in keys {
            if let Some(property) = record.properties.get_mut(key) {
                property.configurable = false;
            }
        }
        record.extensible = false;
        Ok(thrower)
    }

    /// AddRestrictedFunctionProperties (`#sec-addrestrictedfunctionproperties`): "caller" and
    /// "arguments" of `%Function.prototype%`, accessors whose getter and setter are
    /// `%ThrowTypeError%`.
    fn add_restricted_function_properties(
        &mut self,
        function: ObjectRef,
        thrower: ObjectRef,
    ) -> Completion<()> {
        for name in ["caller", "arguments"] {
            let key = self.name_key(name)?;
            self.add_property(
                function,
                key,
                Property {
                    slot: Slot::Accessor {
                        get: Some(thrower),
                        set: Some(thrower),
                    },
                    enumerable: false,
                    configurable: true,
                },
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::host::ErrorKind;
    use crate::host::hooks::test_host::agent_with_realm;
    use crate::host::outcome::Abrupt;
    use crate::object::property::Slot;
    use crate::realm::intrinsics::Intrinsic;
    use crate::value::Value;

    /// `%ThrowTypeError%` (`#sec-%throwtypeerror%`) throws a TypeError, is not extensible, and
    /// is the getter and setter of `%Function.prototype%`'s "caller" and "arguments"
    /// (`#sec-addrestrictedfunctionproperties`); `%Function.prototype%` returns undefined.
    #[test]
    fn throw_type_error_and_restricted_function_properties() {
        let (mut agent, realm) = agent_with_realm();
        let realm = agent.heap.roots.get_realm(&realm.0).unwrap();
        let thrower = agent.intrinsic(realm, Intrinsic::ThrowTypeError).unwrap();
        let function_prototype = agent
            .intrinsic(realm, Intrinsic::FunctionPrototype)
            .unwrap();
        assert_eq!(agent.is_extensible(thrower), Ok(false));
        let thrown = agent.call_value(Value::Object(thrower), Value::Undefined, &[]);
        let Err(Abrupt::Throw(Value::Object(error))) = thrown else {
            panic!("{thrown:?}");
        };
        let type_error = agent
            .intrinsic(realm, Intrinsic::ErrorPrototype(ErrorKind::TypeError))
            .unwrap();
        assert_eq!(agent.get_prototype_of(error), Ok(Some(type_error)));
        for name in ["caller", "arguments"] {
            let key = agent.name_key(name).unwrap();
            let property = agent
                .get_own_property(function_prototype, key)
                .unwrap()
                .unwrap();
            assert_eq!(
                property.slot,
                Slot::Accessor {
                    get: Some(thrower),
                    set: Some(thrower)
                }
            );
            assert!(!property.enumerable && property.configurable);
        }
        assert_eq!(
            agent.call_value(Value::Object(function_prototype), Value::Null, &[]),
            Ok(Value::Undefined)
        );
    }
}
