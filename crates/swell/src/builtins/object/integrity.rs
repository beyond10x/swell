//! The integrity and prototype methods of `Object` and `Object.prototype` (ECMA-262 20.1.2 and
//! 20.1.3, `#sec-properties-of-the-object-constructor`,
//! `#sec-properties-of-the-object-prototype-object`), the group of
//! `story:object-integrity-and-prototype-methods`: on `%Object%`, `Object.create`
//! (`#sec-object.create`), `Object.defineProperties` (`#sec-object.defineproperties`) with
//! ObjectDefineProperties (`#sec-objectdefineproperties`), `Object.freeze`
//! (`#sec-object.freeze`), `Object.is` (`#sec-object.is`), `Object.isExtensible`
//! (`#sec-object.isextensible`), `Object.isFrozen` (`#sec-object.isfrozen`), `Object.isSealed`
//! (`#sec-object.issealed`), `Object.preventExtensions` (`#sec-object.preventextensions`),
//! `Object.seal` (`#sec-object.seal`) and `Object.setPrototypeOf`
//! (`#sec-object.setprototypeof`); on `%Object.prototype%`, `isPrototypeOf`
//! (`#sec-object.prototype.isprototypeof`), `toLocaleString`
//! (`#sec-object.prototype.tolocalestring`, the definition of ECMA-262 without ECMA-402) and
//! `valueOf` (`#sec-object.prototype.valueof`). Each list is in the order of the clauses.
//!
//! The integrity levels are SetIntegrityLevel and TestIntegrityLevel of `ops/objects.rs`, which
//! charge a step per key; `toLocaleString` is Invoke of that file. ObjectDefineProperties
//! charges a step per key in each of its two loops and its two lists (the keys and the
//! descriptors read) to the memory budget; the walk of `isPrototypeOf` charges a step per
//! object it passes. What a builtin holds across a step that may run code (a getter, a
//! `valueOf`) is on the agent's kept stack (`builtins/object/mod.rs`).

use crate::builtins::object::argument;
use crate::builtins::{Method, SymbolData};
use crate::host::agent::Agent;
use crate::host::outcome::{Abrupt, Completion, Termination};
use crate::host::{ErrorKind, Host};
use crate::object::builtin_function::KeptRoom;
use crate::object::ordinary::OrdinaryData;
use crate::object::property::{Descriptor, Key};
use crate::ops::objects::IntegrityLevel;
use crate::value::{ObjectRef, Value};
use alloc::vec::Vec;

/// The builtins of this group, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IntegrityBuiltin {
    /// `Object.create ( proto, properties )` (20.1.2.2).
    Create,
    /// `Object.defineProperties ( obj, properties )` (20.1.2.3).
    DefineProperties,
    /// `Object.freeze ( obj )` (20.1.2.6).
    Freeze,
    /// `Object.is ( value1, value2 )` (20.1.2.15).
    Is,
    /// `Object.isExtensible ( obj )` (20.1.2.16).
    IsExtensible,
    /// `Object.isFrozen ( obj )` (20.1.2.17).
    IsFrozen,
    /// `Object.isSealed ( obj )` (20.1.2.18).
    IsSealed,
    /// `Object.preventExtensions ( obj )` (20.1.2.20).
    PreventExtensions,
    /// `Object.seal ( obj )` (20.1.2.22).
    Seal,
    /// `Object.setPrototypeOf ( obj, proto )` (20.1.2.23).
    SetPrototypeOf,
    /// `Object.prototype.isPrototypeOf ( value )` (20.1.3.3).
    IsPrototypeOf,
    /// `Object.prototype.toLocaleString ( [ reserved1 [ , reserved2 ] ] )` (20.1.3.5).
    ToLocaleString,
    /// `Object.prototype.valueOf ( )` (20.1.3.7).
    ValueOf,
}

/// The methods this group defines on `%Object%`, in the order of their clauses.
pub(crate) const CONSTRUCTOR_METHODS: &[Method<IntegrityBuiltin>] = &[
    ("create", 2, IntegrityBuiltin::Create),
    ("defineProperties", 2, IntegrityBuiltin::DefineProperties),
    ("freeze", 1, IntegrityBuiltin::Freeze),
    ("is", 2, IntegrityBuiltin::Is),
    ("isExtensible", 1, IntegrityBuiltin::IsExtensible),
    ("isFrozen", 1, IntegrityBuiltin::IsFrozen),
    ("isSealed", 1, IntegrityBuiltin::IsSealed),
    ("preventExtensions", 1, IntegrityBuiltin::PreventExtensions),
    ("seal", 1, IntegrityBuiltin::Seal),
    ("setPrototypeOf", 2, IntegrityBuiltin::SetPrototypeOf),
];

/// The methods this group defines on `%Object.prototype%`, in the order of their clauses.
pub(crate) const PROTOTYPE_METHODS: &[Method<IntegrityBuiltin>] = &[
    ("isPrototypeOf", 1, IntegrityBuiltin::IsPrototypeOf),
    ("toLocaleString", 0, IntegrityBuiltin::ToLocaleString),
    ("valueOf", 0, IntegrityBuiltin::ValueOf),
];

/// The data properties keyed by a well-known symbol this group defines on
/// `%Object.prototype%`, after its methods: none.
pub(crate) fn prototype_symbol_data<H: Host>() -> [SymbolData<H>; 0] {
    []
}

/// The values ObjectDefineProperties keeps per key: the descriptor object while it is read,
/// then the descriptor's [[Value]], [[Get]] and [[Set]].
const KEPT_PER_DESCRIPTOR: usize = 4;

/// The bytes ObjectDefineProperties charges for `count` keys beyond the list of keys, which
/// [[OwnPropertyKeys]] charged as every list of keys is ([`crate::object::key_list_bytes`]), and the room on
/// the kept stack ([`Agent::reserve_kept`]): the list of descriptors read.
fn define_properties_bytes(count: usize) -> u64 {
    let per_key = size_of::<(Key, Descriptor)>();
    u64::try_from(count.saturating_mul(per_key)).unwrap_or(u64::MAX)
}

/// What ObjectDefineProperties holds while its loop runs: ToObject of the properties argument,
/// its own keys, the bytes charged for its lists (the list of keys, taken over from
/// [[OwnPropertyKeys]], and that of the descriptors), the mark of the kept stack below the
/// object and the keys, and the room its growth for them took.
struct HeldKeys {
    properties: ObjectRef,
    keys: Vec<Key>,
    bytes: u64,
    mark: usize,
    room: KeptRoom,
}

/// `Some` of the object `value` is, `None` for null, and `Err(())` for any other value: the
/// prototype argument of `Object.create` and `Object.setPrototypeOf`.
fn object_or_null(value: Value) -> Result<Option<ObjectRef>, ()> {
    match value {
        Value::Object(object) => Ok(Some(object)),
        Value::Null => Ok(None),
        _ => Err(()),
    }
}

impl<H: Host> Agent<H> {
    /// The steps of the builtins of this group; not inlined, so that the frame of
    /// `run_object_builtin` does not grow with its arms.
    #[inline(never)]
    pub(crate) fn run_object_integrity_builtin(
        &mut self,
        id: IntegrityBuiltin,
        _function: ObjectRef,
        this: Value,
        args: &[Value],
        _new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        match id {
            IntegrityBuiltin::Create => self.object_create(args),
            IntegrityBuiltin::DefineProperties => {
                self.object_define_properties(argument(args, 0), argument(args, 1))
            }
            IntegrityBuiltin::Freeze => self.object_set_level(args, IntegrityLevel::Frozen),
            IntegrityBuiltin::Is => self.object_is(args),
            IntegrityBuiltin::IsExtensible => self.object_is_extensible(args),
            IntegrityBuiltin::IsFrozen => self.object_test_level(args, IntegrityLevel::Frozen),
            IntegrityBuiltin::IsSealed => self.object_test_level(args, IntegrityLevel::Sealed),
            IntegrityBuiltin::PreventExtensions => self.object_prevent_extensions(args),
            IntegrityBuiltin::Seal => self.object_set_level(args, IntegrityLevel::Sealed),
            IntegrityBuiltin::SetPrototypeOf => self.object_set_prototype_of(args),
            IntegrityBuiltin::IsPrototypeOf => self.object_is_prototype_of(this, args),
            IntegrityBuiltin::ToLocaleString => self.object_to_locale_string(this),
            IntegrityBuiltin::ValueOf => self.object_value_of(this),
        }
    }

    /// `Object.create ( proto, properties )` (`#sec-object.create`).
    #[inline(never)]
    fn object_create(&mut self, args: &[Value]) -> Completion<Value> {
        // 1. If proto is not an Object and proto is not null, throw a TypeError exception.
        let Ok(prototype) = object_or_null(argument(args, 0)) else {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "Object.create: the prototype must be an object or null",
            ));
        };
        // 2. Let obj be OrdinaryObjectCreate(proto).
        let object = self.ordinary_object_create(prototype, OrdinaryData::Plain)?;
        // 3. If properties is not undefined, then return ? ObjectDefineProperties(obj,
        // properties): it may run code, so obj, which is new, is kept.
        let properties = argument(args, 1);
        if matches!(properties, Value::Undefined) {
            // 4. Return obj.
            return Ok(Value::Object(object));
        }
        let keep = self.keep(Value::Object(object));
        let defined = self.object_define_properties(Value::Object(object), properties);
        self.release(keep);
        defined
    }

    /// `Object.defineProperties ( obj, properties )` (`#sec-object.defineproperties`) with
    /// ObjectDefineProperties ( obj, properties ) (`#sec-objectdefineproperties`), which
    /// `Object.create` calls with the object it made; `target` is held by the activation or by
    /// that caller. A getter of `properties` or of a descriptor re-enters from the loop of step
    /// 4, so this function and [`Agent::read_descriptor`], the frames of this group on that
    /// path, hold as little as they can: the steps before and after the loop are functions of
    /// their own, and neither uses `?`, whose temporaries take slots of the frame in a debug
    /// build (`STACK_BYTES_PER_REENTRY`, `host/limits.rs`).
    #[inline(never)]
    #[expect(
        clippy::question_mark,
        reason = "a `?` adds slots to this frame, which every re-entry through a getter pays"
    )]
    fn object_define_properties(&mut self, target: Value, properties: Value) -> Completion<Value> {
        // 20.1.2.3, step 1. If obj is not an Object, throw a TypeError exception.
        let Value::Object(object) = target else {
            return Err(self.define_properties_target_error());
        };
        // ObjectDefineProperties, steps 1 and 2.
        let held = match self.hold_define_properties_keys(properties) {
            Ok(held) => held,
            Err(abrupt) => return Err(abrupt),
        };
        // 3. Let descriptors be a new empty List: room for every key, charged with the keys.
        let mut descriptors: Vec<(Key, Descriptor)> = Vec::new();
        let mut done = self.reserve_descriptors(&mut descriptors, held.keys.len());
        // 4. For each element nextKey of keys, do
        for &key in &held.keys {
            if done.is_err() {
                break;
            }
            done = self.read_descriptor(held.properties, key, &mut descriptors);
        }
        // 5. For each element property of descriptors, do
        if done.is_ok() {
            done = self.define_descriptors(object, &descriptors);
        }
        self.release_define_properties_keys(&held);
        // 6. Return obj.
        done.map(|()| Value::Object(object))
    }

    /// The TypeError of step 1 of `Object.defineProperties`.
    fn define_properties_target_error(&mut self) -> Abrupt {
        self.throw_error(
            ErrorKind::TypeError,
            "Object.defineProperties called on a non-object",
        )
    }

    /// Steps 1 and 2 of ObjectDefineProperties: ToObject of `properties` and its own keys,
    /// both kept, since the object may be new and a getter may delete a key from it while its
    /// atom is still to be defined. The list of keys ([[OwnPropertyKeys]] charged it), the list
    /// of descriptors the loop reads and the room on the kept stack for each key and its values
    /// are charged to the memory budget until [`Agent::release_define_properties_keys`].
    fn hold_define_properties_keys(&mut self, properties: Value) -> Completion<HeldKeys> {
        // 1. Set properties to ? ToObject(properties).
        let properties = self.to_object(properties)?;
        let room = self.kept_room();
        let mark = self.keep(Value::Object(properties));
        // 2. Let keys be ? properties.[[OwnPropertyKeys]](): a step per key, the list charged
        // before it exists; no code runs.
        let list = match self.own_property_keys(properties) {
            Ok(list) => list,
            Err(abrupt) => {
                self.end_kept_room(mark, &room);
                return Err(abrupt);
            }
        };
        let mut held = HeldKeys {
            properties,
            bytes: list.bytes,
            keys: list.keys,
            mark,
            room,
        };
        let descriptors = define_properties_bytes(held.keys.len());
        if self.heap.accounting.charge(descriptors).is_err() {
            self.release_define_properties_keys(&held);
            return Err(self.terminate(Termination::MemoryBudget));
        }
        held.bytes = held.bytes.saturating_add(descriptors);
        let slots = held.keys.len().saturating_mul(KEPT_PER_DESCRIPTOR + 1);
        if let Err(abrupt) = self.reserve_kept(&mut held.room, slots) {
            self.release_define_properties_keys(&held);
            return Err(abrupt);
        }
        for &key in &held.keys {
            self.keep_key(key);
        }
        Ok(held)
    }

    /// Release what [`Agent::hold_define_properties_keys`] kept, give the kept stack back the
    /// room it had and credit what it charged.
    fn release_define_properties_keys(&mut self, held: &HeldKeys) {
        self.end_kept_room(held.mark, &held.room);
        self.heap.accounting.credit(held.bytes);
    }

    /// Room in `descriptors` for `count` of them, so that the loop of step 4 never grows it.
    fn reserve_descriptors(
        &mut self,
        descriptors: &mut Vec<(Key, Descriptor)>,
        count: usize,
    ) -> Completion<()> {
        if descriptors.try_reserve(count).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        Ok(())
    }

    /// Step 4 of ObjectDefineProperties for `key`: a step, then its descriptor appended to
    /// `descriptors` when its property is enumerable. The descriptor object and the
    /// descriptor's values are kept, on the room the caller reserved, until the caller
    /// releases them after step 5.
    #[expect(
        clippy::question_mark,
        reason = "a `?` adds slots to this frame, which every re-entry through a getter pays"
    )]
    fn read_descriptor(
        &mut self,
        properties: ObjectRef,
        key: Key,
        descriptors: &mut Vec<(Key, Descriptor)>,
    ) -> Completion<()> {
        // a. Let propDesc be ? properties.[[GetOwnProperty]](nextKey).
        // b. If propDesc is not undefined and propDesc.[[Enumerable]] is true, then
        match self.is_enumerable_own(properties, key) {
            Ok(true) => {}
            Ok(false) => return Ok(()),
            Err(abrupt) => return Err(abrupt),
        }
        // i. Let descObj be ? Get(properties, nextKey).
        let descriptor_object = match self.get(properties, key) {
            Ok(value) => value,
            Err(abrupt) => return Err(abrupt),
        };
        self.keep(descriptor_object);
        // ii. Let desc be ? ToPropertyDescriptor(descObj).
        match self.to_property_descriptor(descriptor_object) {
            // iii. Append the Record { [[Key]]: nextKey, [[Descriptor]]: desc } to
            // descriptors.
            Ok(descriptor) => {
                self.append_descriptor(key, &descriptor, descriptors);
                Ok(())
            }
            Err(abrupt) => Err(abrupt),
        }
    }

    /// Steps 4.a and 4.b of ObjectDefineProperties, after the step of the loop: whether `key`
    /// is an enumerable own property of `properties`.
    fn is_enumerable_own(&mut self, properties: ObjectRef, key: Key) -> Completion<bool> {
        self.charge_steps(1)?;
        Ok(self
            .get_own_property(properties, key)?
            .is_some_and(|property| property.enumerable))
    }

    /// Step 4.b.iii of ObjectDefineProperties: keep the values of `descriptor` (its [[Value]],
    /// [[Get]] and [[Set]]) and append it with `key`.
    fn append_descriptor(
        &mut self,
        key: Key,
        descriptor: &Descriptor,
        descriptors: &mut Vec<(Key, Descriptor)>,
    ) {
        if let Some(value) = descriptor.value {
            self.keep(value);
        }
        for function in [descriptor.get, descriptor.set]
            .into_iter()
            .flatten()
            .flatten()
        {
            self.keep(Value::Object(function));
        }
        descriptors.push((key, *descriptor));
    }

    /// Step 5 of ObjectDefineProperties: DefinePropertyOrThrow of each descriptor read, in
    /// order. A step per descriptor.
    fn define_descriptors(
        &mut self,
        object: ObjectRef,
        descriptors: &[(Key, Descriptor)],
    ) -> Completion<()> {
        for &(key, descriptor) in descriptors {
            self.charge_steps(1)?;
            // a. Perform ? DefinePropertyOrThrow(obj, property.[[Key]],
            // property.[[Descriptor]]).
            self.define_property_or_throw(object, key, descriptor)?;
        }
        Ok(())
    }

    /// `Object.freeze ( obj )` (`#sec-object.freeze`) and `Object.seal ( obj )`
    /// (`#sec-object.seal`), by `level`.
    fn object_set_level(&mut self, args: &[Value], level: IntegrityLevel) -> Completion<Value> {
        // 1. If obj is not an Object, return obj.
        let value = argument(args, 0);
        let Value::Object(object) = value else {
            return Ok(value);
        };
        // 2. Let status be ? SetIntegrityLevel(obj, level).
        // 3. If status is false, throw a TypeError exception.
        if !self.set_integrity_level(object, level)? {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "the object cannot be made non-extensible",
            ));
        }
        // 4. Return obj.
        Ok(value)
    }

    /// `Object.isFrozen ( obj )` (`#sec-object.isfrozen`) and `Object.isSealed ( obj )`
    /// (`#sec-object.issealed`), by `level`.
    fn object_test_level(&mut self, args: &[Value], level: IntegrityLevel) -> Completion<Value> {
        // 1. If obj is not an Object, return true.
        let Value::Object(object) = argument(args, 0) else {
            return Ok(Value::Boolean(true));
        };
        // 2. Return ? TestIntegrityLevel(obj, level).
        Ok(Value::Boolean(self.test_integrity_level(object, level)?))
    }

    /// `Object.is ( value1, value2 )` (`#sec-object.is`).
    fn object_is(&mut self, args: &[Value]) -> Completion<Value> {
        // 1. Return SameValue(value1, value2).
        Ok(Value::Boolean(
            self.same_value(argument(args, 0), argument(args, 1))?,
        ))
    }

    /// `Object.isExtensible ( obj )` (`#sec-object.isextensible`).
    fn object_is_extensible(&mut self, args: &[Value]) -> Completion<Value> {
        // 1. If obj is not an Object, return false.
        let Value::Object(object) = argument(args, 0) else {
            return Ok(Value::Boolean(false));
        };
        // 2. Return ? IsExtensible(obj).
        Ok(Value::Boolean(self.is_extensible(object)?))
    }

    /// `Object.preventExtensions ( obj )` (`#sec-object.preventextensions`).
    fn object_prevent_extensions(&mut self, args: &[Value]) -> Completion<Value> {
        // 1. If obj is not an Object, return obj.
        let value = argument(args, 0);
        let Value::Object(object) = value else {
            return Ok(value);
        };
        // 2. Let status be ? obj.[[PreventExtensions]]().
        // 3. If status is false, throw a TypeError exception.
        if !self.prevent_extensions(object)? {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "the object cannot be made non-extensible",
            ));
        }
        // 4. Return obj.
        Ok(value)
    }

    /// `Object.setPrototypeOf ( obj, proto )` (`#sec-object.setprototypeof`).
    fn object_set_prototype_of(&mut self, args: &[Value]) -> Completion<Value> {
        // 1. Perform ? RequireObjectCoercible(obj) (`#sec-requireobjectcoercible`).
        let value = argument(args, 0);
        if matches!(value, Value::Undefined | Value::Null) {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "Object.setPrototypeOf called on undefined or null",
            ));
        }
        // 2. If proto is not an Object and proto is not null, throw a TypeError exception.
        let Ok(prototype) = object_or_null(argument(args, 1)) else {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "Object.setPrototypeOf: the prototype must be an object or null",
            ));
        };
        // 3. If obj is not an Object, return obj.
        let Value::Object(object) = value else {
            return Ok(value);
        };
        // 4. Let status be ? obj.[[SetPrototypeOf]](proto).
        // 5. If status is false, throw a TypeError exception.
        if !self.set_prototype_of(object, prototype)? {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "the prototype of the object cannot be set",
            ));
        }
        // 6. Return obj.
        Ok(value)
    }

    /// `Object.prototype.isPrototypeOf ( value )` (`#sec-object.prototype.isprototypeof`). A
    /// step per object the walk passes; [[GetPrototypeOf]] runs no code, so nothing is kept.
    fn object_is_prototype_of(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        // 1. If value is not an Object, return false.
        let Value::Object(mut value) = argument(args, 0) else {
            return Ok(Value::Boolean(false));
        };
        // 2. Let obj be ? ToObject(this value).
        let object = self.to_object(this)?;
        // 3. Repeat,
        loop {
            self.charge_steps(1)?;
            // a. Set value to ? value.[[GetPrototypeOf]]().
            // b. If value is null, return false.
            let Some(prototype) = self.get_prototype_of(value)? else {
                return Ok(Value::Boolean(false));
            };
            // c. If SameValue(obj, value) is true, return true.
            if prototype == object {
                return Ok(Value::Boolean(true));
            }
            value = prototype;
        }
    }

    /// `Object.prototype.toLocaleString ( [ reserved1 [ , reserved2 ] ] )`
    /// (`#sec-object.prototype.tolocalestring`): the this value is held by the activation.
    #[inline(never)]
    fn object_to_locale_string(&mut self, this: Value) -> Completion<Value> {
        // 1. Let thisValue be the this value.
        // 2. Return ? Invoke(thisValue, "toString").
        let key = self.name_key("toString")?;
        self.invoke(this, key, &[])
    }

    /// `Object.prototype.valueOf ( )` (`#sec-object.prototype.valueof`).
    fn object_value_of(&mut self, this: Value) -> Completion<Value> {
        // 1. Return ? ToObject(this value).
        Ok(Value::Object(self.to_object(this)?))
    }
}
