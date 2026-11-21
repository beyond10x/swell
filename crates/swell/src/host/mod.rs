//! The host interface (`docs/design/host-interface.md`; `docs/design/engine.md`, decision 9): the
//! contexts through which host code reaches a running agent, `with_cx`, and the types of
//! decisions 7 and 14. Every other file of `host/` is public through the glob below, so a later
//! story makes a type public by making it `pub` in its file, with no edit of this file or of
//! `lib.rs`.
//!
//! Two contexts (decision 1): `Cx` can run ECMAScript code, `HookCx` can read, create values,
//! parse and finish module loads but runs no code. Both have the methods of
//! [`scoped_methods!`]; the calls that run code are `Cx`'s alone. A method that takes a handle
//! answers `Result<_, HostError>` and checks every handle among its arguments before anything
//! else, before the sticky termination of decision 10 too (decision 5).
//! Written by `story:heap-values-and-host-surface` and `story:object-model-and-realms`.

pub(crate) mod agent;
pub(crate) mod hooks;
pub(crate) mod host_object;
pub(crate) mod interrupt;
pub(crate) mod limits;
pub(crate) mod outcome;

pub use agent::*;
pub use hooks::*;
pub use host_object::*;
pub use interrupt::*;
pub use limits::*;
pub use outcome::*;

use crate::heap::arena::Access;
use crate::object::property::{Descriptor, Key};
use crate::object::{ObjectKind, ObjectRecord};
use crate::value::{ObjectRef, PropertyKey, Realm, RealmRef, Root, ScriptRef, StringRef, Value};

/// The kind of an error object (20.5.5, `#sec-native-error-types-used-in-this-standard`, and
/// `Error`), for `new_error`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    Error,
    EvalError,
    RangeError,
    ReferenceError,
    SyntaxError,
    TypeError,
    UriError,
}

impl ErrorKind {
    /// Every kind, `Error` first.
    pub const ALL: [ErrorKind; 7] = [
        ErrorKind::Error,
        ErrorKind::EvalError,
        ErrorKind::RangeError,
        ErrorKind::ReferenceError,
        ErrorKind::SyntaxError,
        ErrorKind::TypeError,
        ErrorKind::UriError,
    ];

    /// The name of the constructor and the value of its prototype's "name".
    pub fn name(self) -> &'static str {
        match self {
            ErrorKind::Error => "Error",
            ErrorKind::EvalError => "EvalError",
            ErrorKind::RangeError => "RangeError",
            ErrorKind::ReferenceError => "ReferenceError",
            ErrorKind::SyntaxError => "SyntaxError",
            ErrorKind::TypeError => "TypeError",
            ErrorKind::UriError => "URIError",
        }
    }
}

/// The source text of a script (decision 8; `#sec-source-text`: "a sequence of Unicode code
/// points"), as UTF-8, or as UTF-16 code units, lone surrogates included.
#[derive(Clone, Copy, Debug)]
pub enum Source<'s> {
    Utf8(&'s str),
    Utf16(&'s [u16]),
}

/// The preferredType of ToPrimitive (`#sec-toprimitive`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PreferredType {
    String,
    Number,
}

/// A Property Descriptor (6.2.6, `#sec-property-descriptor-specification-type`): each field
/// present (`Some`) or absent (`None`). `get` and `set` hold a function or `Value::Undefined`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PropertyDescriptor {
    pub value: Option<Value>,
    pub writable: Option<bool>,
    pub get: Option<Value>,
    pub set: Option<Value>,
    pub enumerable: Option<bool>,
    pub configurable: Option<bool>,
}

/// What a host function receives (decision 14): `this`, the arguments, `new_target` when it is
/// constructed, and the function object itself.
#[derive(Clone, Copy, Debug)]
pub struct HostCall<'c> {
    pub this: Value,
    pub args: &'c [Value],
    pub new_target: Option<ObjectRef>,
    pub callee: ObjectRef,
}

/// The steps of a host function (decision 14).
pub type HostFn<H> = fn(cx: &mut Cx<'_, H>, call: HostCall<'_>) -> Completion<Value>;

/// What a realm's setup closure gives (decision 7): the global object and, when it is another
/// object, the global `this` value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlobalObjects {
    pub global: ObjectRef,
    pub this: Option<ObjectRef>,
}

/// The setup closure of `create_realm` (decision 7): called once, after the realm's intrinsics
/// exist and before the properties of clause 19 are defined.
pub type GlobalSetup<'g, H> =
    &'g mut dyn FnMut(&mut HookCx<'_, H>, RealmRef) -> Completion<GlobalObjects>;

/// The running agent and its host, while host code runs inside an entry: a `with_cx` closure or
/// a host function. Calls made through it can run code.
pub struct Cx<'a, H: Host> {
    pub(crate) agent: &'a mut Agent<H>,
}

/// The agent inside a hook that runs no code (decision 1): it has no method that calls a
/// function, constructs or evaluates.
pub struct HookCx<'a, H: Host> {
    pub(crate) agent: &'a mut Agent<H>,
}

impl<H: Host> Agent<H> {
    /// A context for creating and converting values while no code runs. A call made through
    /// it is an outermost entry (decision 2); the values it hands out are valid until `f`
    /// returns, and a value kept longer needs a `Root`. A termination of a call inside `f` is
    /// sticky until `f` returns; after `StepBudget`, `MemoryBudget` and `Interrupted` the next
    /// entry runs afresh, after `EngineFault` the agent is faulted.
    ///
    /// `realm` is the current realm of every call made through it (decision 7) and is checked
    /// first: a stale or released one answers `Err(StaleHandle)`, one of another agent
    /// `Err(ForeignHandle)`, and `f` does not run.
    ///
    /// `with_cx` is a safe point: it releases the dropped roots and may collect first. If that
    /// collection finds a stale handle, the agent is faulted and `f` does not run.
    pub fn with_cx<R>(
        &mut self,
        realm: &Realm,
        f: impl FnOnce(&mut Cx<'_, H>) -> R,
    ) -> Result<R, HostError> {
        self.alive()?;
        let realm = self.live_realm(realm)?;
        self.termination = None;
        if self.safe_point().is_err() {
            self.fault();
            return Err(HostError::Faulted);
        }
        let start = self.scope.len();
        let result = self.in_entry_realm(realm, |agent| f(&mut Cx { agent }));
        self.scope.truncate(start);
        self.end_entry();
        Ok(result)
    }

    fn scoped_root(&mut self, value: Value) -> Result<Root, HostError> {
        self.alive()?;
        self.heap.check_value(value).map_err(host_error)?;
        Ok(self.heap.roots.add(value))
    }

    fn scoped_get(&self, root: &Root) -> Result<Value, HostError> {
        self.alive()?;
        self.heap.roots.get(root)
    }

    fn scoped_new_string(&mut self, text: &str) -> Completion<StringRef> {
        self.running()?;
        let string = self.new_string_str(text)?;
        self.hand_out(Value::String(string));
        Ok(string)
    }

    fn scoped_new_string_utf16(&mut self, units: &[u16]) -> Completion<StringRef> {
        self.running()?;
        let string = self.new_string_value(units)?;
        self.hand_out(Value::String(string));
        Ok(string)
    }

    fn scoped_to_utf16(&self, string: StringRef) -> Result<Vec<u16>, HostError> {
        self.alive()?;
        Ok(self
            .heap
            .strings
            .get(string.handle)
            .map_err(host_error)?
            .to_utf16())
    }

    fn scoped_to_rust_string_lossy(&self, string: StringRef) -> Result<String, HostError> {
        self.alive()?;
        Ok(self
            .heap
            .strings
            .get(string.handle)
            .map_err(host_error)?
            .to_rust_string_lossy())
    }

    fn scoped_new_host_object(
        &mut self,
        spec: HostObjectSpec,
    ) -> Result<Completion<ObjectRef>, HostError> {
        self.alive()?;
        if let Some(prototype) = spec.prototype {
            self.check_object(prototype)?;
        }
        Ok(self.running().and_then(|()| {
            let owned = spec.data.as_deref().map_or(0, size_of_val);
            let owned = u64::try_from(owned).unwrap_or(u64::MAX);
            let record = ObjectRecord::new(
                spec.prototype,
                ObjectKind::Host(HostObject {
                    brand: spec.brand,
                    data: spec.data,
                }),
            );
            match self.heap.new_object(record, owned) {
                Ok(object) => {
                    self.hand_out(Value::Object(object));
                    Ok(object)
                }
                Err(_) => Err(self.terminate(Termination::MemoryBudget)),
            }
        }))
    }

    fn scoped_brand(&self, object: ObjectRef) -> Result<Option<Brand>, HostError> {
        self.alive()?;
        let record = self.heap.objects.get(object.handle).map_err(host_error)?;
        Ok(match &record.kind {
            ObjectKind::Host(host) => Some(host.brand),
            _ => None,
        })
    }

    fn scoped_host_data<T: HostData>(
        &self,
        object: ObjectRef,
        brand: Brand,
    ) -> Result<Option<&T>, HostError> {
        self.alive()?;
        let record = self.heap.objects.get(object.handle).map_err(host_error)?;
        Ok(match &record.kind {
            ObjectKind::Host(host) => host.data::<T>(brand),
            _ => None,
        })
    }

    fn scoped_host_data_mut<T: HostData>(
        &mut self,
        object: ObjectRef,
        brand: Brand,
    ) -> Result<Option<&mut T>, HostError> {
        self.alive()?;
        let record = self
            .heap
            .objects
            .get_mut(object.handle)
            .map_err(host_error)?;
        Ok(match &mut record.kind {
            ObjectKind::Host(host) => host.data_mut::<T>(brand),
            _ => None,
        })
    }

    fn scoped_realm_ref(&self, realm: &Realm) -> Result<RealmRef, HostError> {
        self.alive()?;
        Ok(RealmRef {
            handle: self.live_realm(realm)?,
        })
    }

    fn scoped_new_error(
        &mut self,
        realm: RealmRef,
        kind: ErrorKind,
        message: &str,
    ) -> Result<Completion<ObjectRef>, HostError> {
        self.alive()?;
        self.heap.realms.get(realm.handle).map_err(host_error)?;
        Ok(self.running().and_then(|()| {
            let error = self.new_error_in(realm.handle, kind, message)?;
            self.hand_out(Value::Object(error));
            Ok(error)
        }))
    }

    /// `Err` for a handle among `values` that reaches no record (decision 5).
    fn check_values(&self, values: &[Value]) -> Result<(), HostError> {
        for &value in values {
            self.heap.check_value(value).map_err(host_error)?;
        }
        Ok(())
    }

    /// `Err` for an object handle that reaches no record (decision 5).
    fn check_object(&self, object: ObjectRef) -> Result<(), HostError> {
        self.heap
            .objects
            .get(object.handle)
            .map(|_| ())
            .map_err(host_error)
    }

    /// `Err` for a property key whose handle reaches no record (decision 5).
    fn check_key(&self, key: PropertyKey) -> Result<(), HostError> {
        let value = match key {
            PropertyKey::String(string) => Value::String(string),
            PropertyKey::Symbol(symbol) => Value::Symbol(symbol),
        };
        self.check_values(&[value])
    }

    /// The engine's key of the host's `key`.
    fn key_of(&mut self, key: PropertyKey) -> Completion<Key> {
        match key {
            PropertyKey::String(string) => self.key_from_string(string),
            PropertyKey::Symbol(symbol) => Ok(Key::Symbol(symbol)),
        }
    }

    /// The host's key of the engine's `key`, handed out.
    fn property_key_out(&mut self, key: Key) -> Completion<PropertyKey> {
        let key = match self.key_value(key)? {
            Value::Symbol(symbol) => PropertyKey::Symbol(symbol),
            Value::String(string) => PropertyKey::String(string),
            _ => return Err(self.fault()),
        };
        let value = match key {
            PropertyKey::String(string) => Value::String(string),
            PropertyKey::Symbol(symbol) => Value::Symbol(symbol),
        };
        self.hand_out(value);
        Ok(key)
    }

    /// ToPropertyDescriptor's checks (`#sec-topropertydescriptor`, steps 13 to 15) of a
    /// descriptor the host gives: a getter or setter that is neither callable nor undefined,
    /// or a descriptor with both accessor and data fields, throws a `TypeError`.
    fn descriptor_of(&mut self, descriptor: PropertyDescriptor) -> Completion<Descriptor> {
        let accessor =
            |agent: &mut Self, field: Option<Value>| -> Completion<Option<Option<ObjectRef>>> {
                match field {
                    None => Ok(None),
                    Some(Value::Undefined) => Ok(Some(None)),
                    Some(value @ Value::Object(object)) if agent.is_callable(value)? => {
                        Ok(Some(Some(object)))
                    }
                    Some(_) => Err(agent.throw_error(
                        ErrorKind::TypeError,
                        "a getter or setter must be a function or undefined",
                    )),
                }
            };
        let get = accessor(self, descriptor.get)?;
        let set = accessor(self, descriptor.set)?;
        let converted = Descriptor {
            value: descriptor.value,
            writable: descriptor.writable,
            get,
            set,
            enumerable: descriptor.enumerable,
            configurable: descriptor.configurable,
        };
        if converted.is_accessor() && converted.is_data() {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "a property descriptor cannot be both accessor and data",
            ));
        }
        Ok(converted)
    }
}

/// The host's answer for a handle that reaches no record.
fn host_error(access: Access) -> HostError {
    match access {
        Access::Foreign => HostError::ForeignHandle,
        Access::Stale => HostError::StaleHandle,
    }
}

/// The methods `Cx` and `HookCx` share (decisions 1, 4, 7, 14 and 15).
macro_rules! scoped_methods {
    ($cx:ident) => {
        impl<'a, H: Host> $cx<'a, H> {
            /// The host value.
            pub fn host(&self) -> &H {
                &self.agent.host
            }

            /// The host value, to change.
            pub fn host_mut(&mut self) -> &mut H {
                &mut self.agent.host
            }

            /// A root holding `value`, so that it outlives the scope.
            pub fn root(&mut self, value: Value) -> Result<Root, HostError> {
                self.agent.scoped_root(value)
            }

            /// The value `root` holds.
            pub fn get(&self, root: &Root) -> Result<Value, HostError> {
                self.agent.scoped_get(root)
            }

            /// A second root of what `root` holds.
            pub fn duplicate(&mut self, root: &Root) -> Result<Root, HostError> {
                self.agent.duplicate(root)
            }

            /// A new String of the UTF-16 encoding of `text`, taken without loss (HOST-34).
            pub fn new_string(&mut self, text: &str) -> Completion<StringRef> {
                self.agent.scoped_new_string(text)
            }

            /// A new String of exactly `units`, lone surrogates included.
            pub fn new_string_utf16(&mut self, units: &[u16]) -> Completion<StringRef> {
                self.agent.scoped_new_string_utf16(units)
            }

            /// Every code unit of `string`, exactly (Web IDL's `DOMString`, HOST-32).
            pub fn to_utf16(&self, string: StringRef) -> Result<Vec<u16>, HostError> {
                self.agent.scoped_to_utf16(string)
            }

            /// `string` with each lone surrogate replaced by U+FFFD (Web IDL's `USVString`,
            /// HOST-33).
            pub fn to_rust_string_lossy(&self, string: StringRef) -> Result<String, HostError> {
                self.agent.scoped_to_rust_string_lossy(string)
            }

            /// A new host object (decision 15). A stale or foreign prototype answers `Err`.
            pub fn new_host_object(
                &mut self,
                spec: HostObjectSpec,
            ) -> Result<Completion<ObjectRef>, HostError> {
                self.agent.scoped_new_host_object(spec)
            }

            /// The brand of `object`, or `None` when it is not a host object.
            pub fn brand(&self, object: ObjectRef) -> Result<Option<Brand>, HostError> {
                self.agent.scoped_brand(object)
            }

            /// The host data of `object` when its brand is `brand` and its data is a `T`, else
            /// `None` (HOST-108).
            pub fn host_data<T: HostData>(
                &self,
                object: ObjectRef,
                brand: Brand,
            ) -> Result<Option<&T>, HostError> {
                self.agent.scoped_host_data(object, brand)
            }

            /// As `host_data`, to change.
            pub fn host_data_mut<T: HostData>(
                &mut self,
                object: ObjectRef,
                brand: Brand,
            ) -> Result<Option<&mut T>, HostError> {
                self.agent.scoped_host_data_mut(object, brand)
            }

            /// The scoped handle of `realm`.
            pub fn realm_ref(&self, realm: &Realm) -> Result<RealmRef, HostError> {
                self.agent.scoped_realm_ref(realm)
            }

            /// A new error object of `kind` of `realm` whose "message" is `message`, as
            /// `new kind(message)` in that realm makes one (20.5).
            pub fn new_error(
                &mut self,
                realm: RealmRef,
                kind: ErrorKind,
                message: &str,
            ) -> Result<Completion<ObjectRef>, HostError> {
                self.agent.scoped_new_error(realm, kind, message)
            }
        }
    };
}

scoped_methods!(Cx);
scoped_methods!(HookCx);

/// The operations of `Cx` that can run code (decision 14). Each checks its handles first, then
/// the sticky termination; the values it answers, thrown ones included, are handed out.
impl<H: Host> Cx<'_, H> {
    /// A host function of `realm` (decision 14): CreateBuiltinFunction
    /// (`#sec-createbuiltinfunction`) with `name` and `length`, whose steps are `steps`, a
    /// constructor when `constructor`, holding `data`.
    pub fn new_host_function(
        &mut self,
        realm: RealmRef,
        name: &str,
        length: u32,
        steps: HostFn<H>,
        constructor: bool,
        data: Option<Box<dyn HostData>>,
    ) -> Result<Completion<ObjectRef>, HostError> {
        let agent = &mut *self.agent;
        agent.alive()?;
        agent.heap.realms.get(realm.handle).map_err(host_error)?;
        Ok(agent.running().and_then(|()| {
            let function =
                agent.create_host_function(realm.handle, name, length, steps, constructor, data)?;
            agent.hand_out(Value::Object(function));
            Ok(function)
        }))
    }

    /// Call (`#sec-call`): a value that is not callable throws a `TypeError`. A re-entry when
    /// code runs (decision 14).
    pub fn call(
        &mut self,
        function: Value,
        this: Value,
        args: &[Value],
    ) -> Result<Completion<Value>, HostError> {
        let agent = &mut *self.agent;
        agent.alive()?;
        agent.check_values(&[function, this])?;
        agent.check_values(args)?;
        let completion = agent
            .begin()
            .and_then(|()| agent.call_value(function, this, args));
        Ok(agent.hand_out_completion(completion))
    }

    /// Construct (`#sec-construct`): a value that is not a constructor throws a `TypeError`;
    /// `new_target` is `function` when `None`, and one that is not a constructor answers
    /// `Err(WrongKind)`.
    pub fn construct(
        &mut self,
        function: Value,
        args: &[Value],
        new_target: Option<Value>,
    ) -> Result<Completion<Value>, HostError> {
        let agent = &mut *self.agent;
        agent.alive()?;
        agent.check_values(&[function])?;
        agent.check_values(args)?;
        let new_target = match new_target {
            Some(value) => {
                agent.check_values(&[value])?;
                Some(agent.constructor_or_wrong_kind(value)?)
            }
            None => None,
        };
        let completion = agent
            .begin()
            .and_then(|()| agent.construct_checked(function, args, new_target));
        Ok(agent.hand_out_completion(completion))
    }

    /// Get (`#sec-get-o-p`). Named `get_property` because `get` reads a root (decision 4).
    pub fn get_property(
        &mut self,
        object: ObjectRef,
        key: PropertyKey,
    ) -> Result<Completion<Value>, HostError> {
        let agent = &mut *self.agent;
        agent.alive()?;
        agent.check_object(object)?;
        agent.check_key(key)?;
        let completion = agent.begin().and_then(|()| {
            let key = agent.key_of(key)?;
            agent.get(object, key)
        });
        Ok(agent.hand_out_completion(completion))
    }

    /// Set (`#sec-set-o-p-v-throw`): whether the property was set; with `throw`, a refusal
    /// throws a `TypeError`.
    pub fn set(
        &mut self,
        object: ObjectRef,
        key: PropertyKey,
        value: Value,
        throw: bool,
    ) -> Result<Completion<bool>, HostError> {
        let agent = &mut *self.agent;
        agent.alive()?;
        agent.check_object(object)?;
        agent.check_key(key)?;
        agent.check_values(&[value])?;
        let completion = agent.begin().and_then(|()| {
            let key = agent.key_of(key)?;
            agent.set(object, key, value, throw)
        });
        Ok(hand_out_throw(agent, completion))
    }

    /// DefinePropertyOrThrow (`#sec-definepropertyorthrow`), with ToPropertyDescriptor's checks
    /// of `descriptor`.
    pub fn define_property_or_throw(
        &mut self,
        object: ObjectRef,
        key: PropertyKey,
        descriptor: PropertyDescriptor,
    ) -> Result<Completion<()>, HostError> {
        let agent = &mut *self.agent;
        agent.alive()?;
        agent.check_object(object)?;
        agent.check_key(key)?;
        let fields = [descriptor.value, descriptor.get, descriptor.set];
        agent.check_values(&fields.into_iter().flatten().collect::<Vec<_>>())?;
        let completion = agent.begin().and_then(|()| {
            let key = agent.key_of(key)?;
            let descriptor = agent.descriptor_of(descriptor)?;
            agent.define_property_or_throw(object, key, descriptor)
        });
        Ok(hand_out_throw(agent, completion))
    }

    /// HasProperty (`#sec-hasproperty`).
    pub fn has_property(
        &mut self,
        object: ObjectRef,
        key: PropertyKey,
    ) -> Result<Completion<bool>, HostError> {
        let agent = &mut *self.agent;
        agent.alive()?;
        agent.check_object(object)?;
        agent.check_key(key)?;
        let completion = agent.begin().and_then(|()| {
            let key = agent.key_of(key)?;
            agent.has_property(object, key)
        });
        Ok(hand_out_throw(agent, completion))
    }

    /// DeletePropertyOrThrow (`#sec-deletepropertyorthrow`).
    pub fn delete_property_or_throw(
        &mut self,
        object: ObjectRef,
        key: PropertyKey,
    ) -> Result<Completion<()>, HostError> {
        let agent = &mut *self.agent;
        agent.alive()?;
        agent.check_object(object)?;
        agent.check_key(key)?;
        let completion = agent.begin().and_then(|()| {
            let key = agent.key_of(key)?;
            agent.delete_property_or_throw(object, key)
        });
        Ok(hand_out_throw(agent, completion))
    }

    /// ToPrimitive (`#sec-toprimitive`).
    pub fn to_primitive(
        &mut self,
        value: Value,
        preferred: Option<PreferredType>,
    ) -> Result<Completion<Value>, HostError> {
        let agent = &mut *self.agent;
        agent.alive()?;
        agent.check_values(&[value])?;
        let completion = agent
            .begin()
            .and_then(|()| agent.to_primitive(value, preferred));
        Ok(agent.hand_out_completion(completion))
    }

    /// ToNumber (`#sec-tonumber`).
    pub fn to_number(&mut self, value: Value) -> Result<Completion<f64>, HostError> {
        let agent = &mut *self.agent;
        agent.alive()?;
        agent.check_values(&[value])?;
        let completion = agent.begin().and_then(|()| agent.to_number(value));
        Ok(hand_out_throw(agent, completion))
    }

    /// ToNumeric (`#sec-tonumeric`).
    pub fn to_numeric(&mut self, value: Value) -> Result<Completion<Value>, HostError> {
        let agent = &mut *self.agent;
        agent.alive()?;
        agent.check_values(&[value])?;
        let completion = agent.begin().and_then(|()| agent.to_numeric(value));
        Ok(agent.hand_out_completion(completion))
    }

    /// ToString (`#sec-tostring`).
    pub fn to_string(&mut self, value: Value) -> Result<Completion<StringRef>, HostError> {
        let agent = &mut *self.agent;
        agent.alive()?;
        agent.check_values(&[value])?;
        let completion = agent.begin().and_then(|()| agent.to_string(value));
        let completion = hand_out_throw(agent, completion);
        if let Ok(string) = completion {
            agent.hand_out(Value::String(string));
        }
        Ok(completion)
    }

    /// ToObject (`#sec-toobject`), with the prototypes of the current realm.
    pub fn to_object(&mut self, value: Value) -> Result<Completion<ObjectRef>, HostError> {
        let agent = &mut *self.agent;
        agent.alive()?;
        agent.check_values(&[value])?;
        let completion = agent.begin().and_then(|()| agent.to_object(value));
        let completion = hand_out_throw(agent, completion);
        if let Ok(object) = completion {
            agent.hand_out(Value::Object(object));
        }
        Ok(completion)
    }

    /// ToPropertyKey (`#sec-topropertykey`).
    pub fn to_property_key(&mut self, value: Value) -> Result<Completion<PropertyKey>, HostError> {
        let agent = &mut *self.agent;
        agent.alive()?;
        agent.check_values(&[value])?;
        let completion = agent.begin().and_then(|()| {
            let key = agent.to_property_key(value)?;
            agent.property_key_out(key)
        });
        Ok(hand_out_throw(agent, completion))
    }

    /// ParseScript (`#sec-parse-script`) of `source` in `realm` (decision 8, over scoped
    /// handles): the script as a `ScriptRef` valid until the outermost entry ends, or the parse
    /// errors as roots of `SyntaxError` objects of `realm`. It runs no code, spends no step and
    /// reads no interrupt; a run already terminated answers its termination.
    pub fn parse_script(
        &mut self,
        realm: RealmRef,
        source: Source<'_>,
        host_defined: Option<HostDefined>,
    ) -> Result<Parsed<ScriptRef>, HostError> {
        let agent = &mut *self.agent;
        agent.alive()?;
        agent.heap.realms.get(realm.handle).map_err(host_error)?;
        if let Err(abrupt) = agent.running() {
            return Ok(Parsed::Terminated(crate::script::parse_termination(abrupt)));
        }
        Ok(
            match agent.parse_script_in(realm.handle, source, host_defined) {
                Ok(crate::script::ParseAnswer::Script(handle)) => {
                    agent.scoped_scripts.push(handle);
                    Parsed::Parsed(ScriptRef { handle })
                }
                Ok(crate::script::ParseAnswer::Errors(errors)) => Parsed::Errors(
                    errors
                        .into_iter()
                        .map(|error| agent.heap.roots.add(error))
                        .collect(),
                ),
                Err(abrupt) => {
                    let termination = crate::script::parse_termination(abrupt);
                    agent.terminate(termination);
                    Parsed::Terminated(termination)
                }
            },
        )
    }

    /// ScriptEvaluation (`#sec-runtime-semantics-scriptevaluation`) of `script`: its completion
    /// value. Inside a run it is a re-entry, counted against `reentry_depth`, whose steps the
    /// outermost entry spends (HTML calls scripts re-entrantly,
    /// `html:clean-up-after-running-script`).
    pub fn evaluate_script(&mut self, script: ScriptRef) -> Result<Completion<Value>, HostError> {
        let agent = &mut *self.agent;
        agent.alive()?;
        agent.heap.scripts.get(script.handle).map_err(host_error)?;
        let completion = agent
            .begin()
            .and_then(|()| agent.script_evaluation(script.handle));
        Ok(agent.hand_out_completion(completion))
    }
}

/// Hand out the thrown value of `completion`, if it is a throw, and answer it.
fn hand_out_throw<H: Host, T>(agent: &mut Agent<H>, completion: Completion<T>) -> Completion<T> {
    if let Err(Abrupt::Throw(value)) = completion {
        agent.hand_out(value);
    }
    completion
}
