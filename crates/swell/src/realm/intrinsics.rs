//! The intrinsics of a realm (ECMA-262 9.3.2, `#sec-createintrinsics`; 6.1.7.4,
//! `#sec-well-known-intrinsic-objects`) that this part has: `%Object.prototype%`,
//! `%Function.prototype%`, `%ThrowTypeError%`, `%Array.prototype%`, `%String.prototype%`,
//! `%Boolean.prototype%`, `%Number.prototype%`, `%Error%`, `%Error.prototype%` and the six
//! NativeErrors with their prototypes, written by `story:object-model-and-realms`; `%Object%`,
//! `%Array%`, `%String%`, `%Math%` and `%JSON%` with the methods the harness files call, written
//! by `story:harness-builtins` (each builtin module installs its own, `builtins/`); `%Number%`,
//! `%Boolean%`, `%isNaN%` and `%isFinite%`, with the "constructor" properties of
//! `%Number.prototype%` and `%Boolean.prototype%` and their methods, taken early by slice 2
//! (`story:number-boolean-and-global-functions-early`); `%eval%` and `%Function%` with
//! `Function.prototype.constructor`, written by `story:eval-and-function-constructor`;
//! `%decodeURI%`, `%decodeURIComponent%`, `%encodeURI%` and `%encodeURIComponent%` (19.2.6,
//! `#sec-uri-handling-functions`), written by `story:global-this-uri-functions-and-is-error`;
//! `%Symbol%` and `%Symbol.prototype%` (20.4, `#sec-symbol-objects`), written by
//! `story:symbol-constructor-and-well-known-symbols`.
//! The other constructors and methods are later slices.
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
use crate::object::property::{Key, Property, Slot};
use crate::realm::WellKnown;
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
    /// `%ThrowTypeError%`: the `callee` accessor of an unmapped arguments object
    /// (`#sec-createunmappedargumentsobject`).
    ThrowTypeError,
    /// `%Error%` and the NativeError constructors.
    Error(ErrorKind),
    /// `%Error.prototype%` and the NativeError prototypes.
    ErrorPrototype(ErrorKind),
    /// `%Object%` (20.1.1).
    Object,
    /// `%Array%` (23.1.1).
    Array,
    /// `%String%` (22.1.1).
    String,
    /// `%Math%` (21.3).
    Math,
    /// `%JSON%` (25.5).
    Json,
    /// `%Number%` (21.1.1), taken early by slice 2.
    Number,
    /// `%Boolean%` (20.3.1), taken early by slice 2.
    Boolean,
    /// `%isNaN%` (19.2.3), taken early by slice 2.
    IsNaN,
    /// `%isFinite%` (19.2.2), taken early by slice 2.
    IsFinite,
    /// `%eval%` (19.2.1), slice 2.
    Eval,
    /// `%Function%` (20.2.1), the constructor that slice 2 delivers.
    Function,
    /// `%Object.prototype.toString%` (20.1.3.6), which `Array.prototype.toString` calls when
    /// its object's "join" is not callable (`#sec-array.prototype.tostring`, step 3).
    ObjectPrototypeToString,
    /// `%decodeURI%` (19.2.6.1, `#sec-decodeuri-encodeduri`).
    DecodeUri,
    /// `%decodeURIComponent%` (19.2.6.2, `#sec-decodeuricomponent-encodeduricomponent`).
    DecodeUriComponent,
    /// `%encodeURI%` (19.2.6.3, `#sec-encodeuri-uri`).
    EncodeUri,
    /// `%encodeURIComponent%` (19.2.6.4, `#sec-encodeuricomponent-uricomponent`).
    EncodeUriComponent,
    /// `%Symbol%` (20.4.1), `story:symbol-constructor-and-well-known-symbols`.
    Symbol,
    /// `%Symbol.prototype%` (20.4.3).
    SymbolPrototype,
}

/// What the builtins of a realm are made with while CreateIntrinsics runs, before the realm
/// record holds its intrinsics: the realm, the two prototypes every builtin needs, and the
/// well-known symbols.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Making {
    pub(crate) realm: Handle,
    pub(crate) object_prototype: ObjectRef,
    pub(crate) function_prototype: ObjectRef,
    pub(crate) symbols: WellKnown,
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
    object: ObjectRef,
    array: ObjectRef,
    string: ObjectRef,
    math: ObjectRef,
    json: ObjectRef,
    number: ObjectRef,
    boolean: ObjectRef,
    is_nan: ObjectRef,
    is_finite: ObjectRef,
    eval: ObjectRef,
    function: ObjectRef,
    object_prototype_to_string: ObjectRef,
    uri: crate::builtins::global::UriFunctions,
    symbol: ObjectRef,
    symbol_prototype: ObjectRef,
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
            Intrinsic::Object => self.object,
            Intrinsic::Array => self.array,
            Intrinsic::String => self.string,
            Intrinsic::Math => self.math,
            Intrinsic::Json => self.json,
            Intrinsic::Number => self.number,
            Intrinsic::Boolean => self.boolean,
            Intrinsic::IsNaN => self.is_nan,
            Intrinsic::IsFinite => self.is_finite,
            Intrinsic::Eval => self.eval,
            Intrinsic::Function => self.function,
            Intrinsic::ObjectPrototypeToString => self.object_prototype_to_string,
            Intrinsic::DecodeUri => self.uri.decode_uri,
            Intrinsic::DecodeUriComponent => self.uri.decode_uri_component,
            Intrinsic::EncodeUri => self.uri.encode_uri,
            Intrinsic::EncodeUriComponent => self.uri.encode_uri_component,
            Intrinsic::Symbol => self.symbol,
            Intrinsic::SymbolPrototype => self.symbol_prototype,
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
            self.object,
            self.array,
            self.string,
            self.math,
            self.json,
            self.number,
            self.boolean,
            self.is_nan,
            self.is_finite,
            self.eval,
            self.function,
            self.object_prototype_to_string,
            self.uri.decode_uri,
            self.uri.decode_uri_component,
            self.uri.encode_uri,
            self.uri.encode_uri_component,
            self.symbol,
            self.symbol_prototype,
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

    /// A built-in function named `name` of `length` whose steps are `id`, defined on `object` as
    /// a method (clause 18: { [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]: true
    /// }); not a constructor (clause 18: "Built-in function objects that are not identified as
    /// constructors do not implement [[Construct]]").
    pub(crate) fn define_builtin_method(
        &mut self,
        making: Making,
        object: ObjectRef,
        name: &str,
        length: u32,
        id: BuiltinId,
    ) -> Completion<ObjectRef> {
        let function = self.create_builtin_function(
            Behaviour::Builtin(id),
            length,
            name,
            making.realm,
            Some(making.function_prototype),
            false,
        )?;
        self.define_builtin_property(object, name, Value::Object(function))?;
        Ok(function)
    }

    /// A built-in function named `name` of `length` whose steps are `id`, for a property the
    /// caller defines itself (a Symbol-keyed method or an accessor).
    pub(crate) fn create_builtin(
        &mut self,
        making: Making,
        name: &str,
        length: u32,
        id: BuiltinId,
    ) -> Completion<ObjectRef> {
        self.create_builtin_function(
            Behaviour::Builtin(id),
            length,
            name,
            making.realm,
            Some(making.function_prototype),
            false,
        )
    }

    /// A built-in constructor named `name` of `length` whose steps are `id`, with its
    /// "prototype" `prototype` ({ [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]:
    /// false }, as 20.1.2.21, 22.1.2.3 and 23.1.2.5 give it) and the prototype's "constructor"
    /// (20.1.3.1, 22.1.3.6, 23.1.3.3).
    pub(crate) fn create_builtin_constructor(
        &mut self,
        making: Making,
        name: &str,
        length: u32,
        id: BuiltinId,
        prototype: ObjectRef,
    ) -> Completion<ObjectRef> {
        let constructor = self.create_builtin_function(
            Behaviour::Builtin(id),
            length,
            name,
            making.realm,
            Some(making.function_prototype),
            true,
        )?;
        let key = self.name_key("prototype")?;
        self.add_property(
            constructor,
            key,
            Property::data(Value::Object(prototype), false, false, false),
        )?;
        self.define_builtin_property(prototype, "constructor", Value::Object(constructor))?;
        Ok(constructor)
    }

    /// The @@toStringTag property of `object`, `tag`, with { [[Writable]]: false,
    /// [[Enumerable]]: false, [[Configurable]]: true } (21.3.1.9, 25.5.5).
    pub(crate) fn define_to_string_tag(
        &mut self,
        making: Making,
        object: ObjectRef,
        tag: &str,
    ) -> Completion<()> {
        let tag = self.new_string_str(tag)?;
        self.add_property(
            object,
            Key::Symbol(making.symbols.to_string_tag),
            Property::data(Value::String(tag), false, false, true),
        )
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
        // Every realm record holds the well-known symbols from its creation (`realm/mod.rs`).
        let Some(symbols) = self.realm_record(realm)?.symbols else {
            return Err(self.fault());
        };
        let making = Making {
            realm,
            object_prototype,
            function_prototype,
            symbols,
        };
        let object = self.create_object_constructor(making)?;
        // %Object.prototype.toString% (20.1.3.6): the method `create_object_constructor` has
        // just defined on %Object.prototype%, read before any code runs.
        let to_string_key = self.name_key("toString")?;
        let Value::Object(object_prototype_to_string) =
            self.get(object_prototype, to_string_key)?
        else {
            return Err(self.fault());
        };
        self.install_function_prototype_methods(making)?;
        let array = self.create_array_constructor(making, array_prototype)?;
        let string = self.create_string_constructor(making, string_prototype)?;
        let math = self.create_math_object(making)?;
        let json = self.create_json_object(making)?;
        let number = self.create_number_constructor(making, number_prototype)?;
        let boolean = self.create_boolean_constructor(making, boolean_prototype)?;
        let (is_finite, is_nan) = self.create_global_functions(making)?;
        let eval = self.create_eval_function(making)?;
        let function = self.create_function_constructor(making)?;
        let uri = self.create_uri_functions(making)?;
        let (symbol, symbol_prototype) = self.create_symbol_constructor(making)?;
        let intrinsics = Intrinsics {
            object_prototype,
            function_prototype,
            array_prototype,
            string_prototype,
            boolean_prototype,
            number_prototype,
            throw_type_error,
            errors,
            object,
            array,
            string,
            math,
            json,
            number,
            boolean,
            is_nan,
            is_finite,
            eval,
            function,
            object_prototype_to_string,
            uri,
            symbol,
            symbol_prototype,
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
