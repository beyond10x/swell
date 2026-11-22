//! The enumeration and copy methods of `Object` (ECMA-262 20.1.2,
//! `#sec-properties-of-the-object-constructor`), the group of
//! `story:object-enumeration-and-copy-methods`: `Object.assign` (`#sec-object.assign`),
//! `Object.entries` (`#sec-object.entries`), `Object.getOwnPropertyDescriptors`
//! (`#sec-object.getownpropertydescriptors`), `Object.hasOwn` (`#sec-object.hasown`),
//! `Object.keys` (`#sec-object.keys`) and `Object.values` (`#sec-object.values`). `keys`,
//! `values` and `entries` call EnumerableOwnProperties (`#sec-enumerableownproperties`) of
//! `ops/objects.rs`. The group defines nothing on `%Object.prototype%`.
//!
//! `Object.getOwnPropertySymbols` (`#sec-object.getownpropertysymbols`) is not here before
//! slice 6: its answers are Symbol values, and before `%Symbol.prototype%` a Symbol value a
//! script holds would fault the agent at its first ToObject (`docs/design/engine.md`, section
//! 12, "A part before its slice": a builtin that exists is correct for every argument). None of
//! the six hands a Symbol value to a script: `assign` and `getOwnPropertyDescriptors` copy and
//! describe a Symbol-keyed property under its key, `keys`, `values` and `entries` skip Symbol
//! keys, and `hasOwn` takes a key and answers a Boolean.
//!
//! Every loop runs for the script and charges a step per key; a list of keys or of results is
//! charged to the memory budget while it is held and credited when it is dropped. What a
//! builtin holds across a step that may run code (a getter, a setter, a toString) is on the
//! kept stack: the object ToObject made, the keys (their atoms are held weakly), the target
//! and the results (`builtins/object/mod.rs`).

use crate::builtins::object::argument;
use crate::builtins::{Method, SymbolData};
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::{Completion, Termination};
use crate::object::ordinary::OrdinaryData;
use crate::object::property::Key;
use crate::ops::objects::PropertyKind;
use crate::realm::intrinsics::Intrinsic;
use crate::value::{ObjectRef, Value};

/// The builtins of this group, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EnumerationBuiltin {
    /// `Object.assign ( target, ...sources )` (20.1.2.1).
    Assign,
    /// `Object.entries ( obj )` (20.1.2.5).
    Entries,
    /// `Object.getOwnPropertyDescriptors ( obj )` (20.1.2.9).
    GetOwnPropertyDescriptors,
    /// `Object.hasOwn ( obj, key )` (20.1.2.14).
    HasOwn,
    /// `Object.keys ( obj )` (20.1.2.19).
    Keys,
    /// `Object.values ( obj )` (20.1.2.24).
    Values,
}

/// The methods this group defines on `%Object%`, in the order of their clauses.
pub(crate) const CONSTRUCTOR_METHODS: &[Method<EnumerationBuiltin>] = &[
    ("assign", 2, EnumerationBuiltin::Assign),
    ("entries", 1, EnumerationBuiltin::Entries),
    (
        "getOwnPropertyDescriptors",
        1,
        EnumerationBuiltin::GetOwnPropertyDescriptors,
    ),
    ("hasOwn", 2, EnumerationBuiltin::HasOwn),
    ("keys", 1, EnumerationBuiltin::Keys),
    ("values", 1, EnumerationBuiltin::Values),
];

/// The methods this group defines on `%Object.prototype%`: none.
pub(crate) const PROTOTYPE_METHODS: &[Method<EnumerationBuiltin>] = &[];

/// The data properties keyed by a well-known symbol this group defines on
/// `%Object.prototype%`, after its methods: none.
pub(crate) fn prototype_symbol_data<H: Host>() -> [SymbolData<H>; 0] {
    []
}

/// The bytes of a list of `count` items of `T`, as a held list is charged.
fn list_bytes<T>(count: usize) -> u64 {
    u64::try_from(count.saturating_mul(size_of::<T>())).unwrap_or(u64::MAX)
}

impl<H: Host> Agent<H> {
    /// The steps of the builtins of this group; not inlined, so that the frame of
    /// `run_object_builtin` does not grow with its arms.
    #[inline(never)]
    pub(crate) fn run_object_enumeration_builtin(
        &mut self,
        id: EnumerationBuiltin,
        _function: ObjectRef,
        _this: Value,
        args: &[Value],
        _new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        // Everything a builtin keeps is released to this mark, whatever it answers.
        let mark = self.kept.len();
        let result = match id {
            EnumerationBuiltin::Assign => self.object_assign(args),
            EnumerationBuiltin::Entries => {
                self.object_enumerable_own(argument(args, 0), PropertyKind::KeyValue)
            }
            EnumerationBuiltin::GetOwnPropertyDescriptors => {
                self.object_get_own_property_descriptors(argument(args, 0))
            }
            EnumerationBuiltin::HasOwn => self.object_has_own(args),
            EnumerationBuiltin::Keys => {
                self.object_enumerable_own(argument(args, 0), PropertyKind::Key)
            }
            EnumerationBuiltin::Values => {
                self.object_enumerable_own(argument(args, 0), PropertyKind::Value)
            }
        };
        self.release(mark);
        result
    }

    /// Charge `bytes` of a held list to the memory budget.
    fn charge_list(&mut self, bytes: u64) -> Completion<()> {
        if self.heap.accounting.charge(bytes).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        Ok(())
    }

    /// The own keys of `object` by [[OwnPropertyKeys]] (a step per key), charged to the memory
    /// budget and kept: answers the keys and the bytes the caller credits when it drops them.
    fn held_keys(&mut self, object: ObjectRef) -> Completion<(Vec<Key>, u64)> {
        let keys = self.own_property_keys(object)?;
        let bytes = list_bytes::<Key>(keys.len());
        self.charge_list(bytes)?;
        if self.kept.try_reserve(keys.len()).is_err() {
            self.heap.accounting.credit(bytes);
            return Err(self.terminate(Termination::MemoryBudget));
        }
        for &key in &keys {
            self.keep_key(key);
        }
        Ok((keys, bytes))
    }

    /// `Object.keys ( obj )` (`#sec-object.keys`), `Object.values ( obj )`
    /// (`#sec-object.values`) and `Object.entries ( obj )` (`#sec-object.entries`), by `kind`,
    /// the three steps of each clause: ToObject, EnumerableOwnProperties of the kind, then
    /// CreateArrayFromList. EnumerableOwnProperties charges a step per key and keeps the keys
    /// and the results; the list is charged to the memory budget until the array is made,
    /// which CreateArrayFromList charges a step per element.
    fn object_enumerable_own(&mut self, value: Value, kind: PropertyKind) -> Completion<Value> {
        // 1. Let coerced be ? ToObject(obj): new for a primitive, so kept across each Get.
        let coerced = self.to_object(value)?;
        self.keep(Value::Object(coerced));
        // 2. Let list be ? EnumerableOwnProperties(coerced, kind).
        let list = self.enumerable_own_properties(coerced, kind)?;
        let bytes = list_bytes::<Value>(list.len());
        self.charge_list(bytes)?;
        // 3. Return CreateArrayFromList(list).
        let array = self.create_array_from_list(&list);
        self.heap.accounting.credit(bytes);
        Ok(Value::Object(array?))
    }

    /// `Object.assign ( target, ...sources )` (`#sec-object.assign`).
    fn object_assign(&mut self, args: &[Value]) -> Completion<Value> {
        // 1. Let to be ? ToObject(target): new for a primitive, so kept across each Set.
        let to = self.to_object(argument(args, 0))?;
        self.keep(Value::Object(to));
        // 2. If only one argument was passed, return to.
        // 3. For each element nextSource of sources, do
        for &next_source in args.iter().skip(1) {
            // a. If nextSource is neither undefined nor null, then
            if matches!(next_source, Value::Undefined | Value::Null) {
                continue;
            }
            let mark = self.kept.len();
            let copied = self.assign_from(to, next_source);
            self.release(mark);
            copied?;
        }
        // 4. Return to.
        Ok(Value::Object(to))
    }

    /// Steps 3.a.i to 3.a.iii of `Object.assign`: copy the enumerable own properties of
    /// `source`, which is neither undefined nor null, to `to`. The caller releases what this
    /// keeps.
    fn assign_from(&mut self, to: ObjectRef, source: Value) -> Completion<()> {
        // i. Let from be ! ToObject(nextSource).
        let from = self.to_object(source)?;
        self.keep(Value::Object(from));
        // ii. Let keys be ? from.[[OwnPropertyKeys]]().
        let (keys, bytes) = self.held_keys(from)?;
        let copied = self.assign_keys(to, from, &keys);
        self.heap.accounting.credit(bytes);
        copied
    }

    /// Step 3.a.iii of `Object.assign`: for each element nextKey of keys, a step each.
    fn assign_keys(&mut self, to: ObjectRef, from: ObjectRef, keys: &[Key]) -> Completion<()> {
        for &next_key in keys {
            self.charge_steps(1)?;
            // 1. Let desc be ? from.[[GetOwnProperty]](nextKey).
            // 2. If desc is not undefined and desc.[[Enumerable]] is true, then
            if !self
                .get_own_property(from, next_key)?
                .is_some_and(|property| property.enumerable)
            {
                continue;
            }
            // a. Let propValue be ? Get(from, nextKey): kept across the Set, which may run a
            // setter.
            let value = self.get(from, next_key)?;
            let mark = self.keep(value);
            // b. Perform ? Set(to, nextKey, propValue, true).
            let set = self.set(to, next_key, value, true);
            self.release(mark);
            set?;
        }
        Ok(())
    }

    /// `Object.getOwnPropertyDescriptors ( obj )` (`#sec-object.getownpropertydescriptors`).
    fn object_get_own_property_descriptors(&mut self, value: Value) -> Completion<Value> {
        // 1. Let coerced be ? ToObject(obj).
        let coerced = self.to_object(value)?;
        self.keep(Value::Object(coerced));
        // 2. Let ownKeys be ? coerced.[[OwnPropertyKeys]]().
        let (own_keys, bytes) = self.held_keys(coerced)?;
        let descriptors = self.descriptors_of(coerced, &own_keys);
        self.heap.accounting.credit(bytes);
        descriptors
    }

    /// Steps 3 to 5 of `Object.getOwnPropertyDescriptors`.
    fn descriptors_of(&mut self, coerced: ObjectRef, own_keys: &[Key]) -> Completion<Value> {
        // 3. Let descriptors be OrdinaryObjectCreate(%Object.prototype%).
        let prototype = self.current_intrinsic(Intrinsic::ObjectPrototype)?;
        let descriptors = self.ordinary_object_create(Some(prototype), OrdinaryData::Plain)?;
        self.keep(Value::Object(descriptors));
        // 4. For each element key of ownKeys, a step each:
        for &key in own_keys {
            self.charge_steps(1)?;
            // a. Let desc be ? coerced.[[GetOwnProperty]](key).
            let property = self.get_own_property(coerced, key)?;
            // b. Let descriptor be FromPropertyDescriptor(desc).
            let descriptor = self.from_property_descriptor(property)?;
            // c. If descriptor is not undefined, perform ! CreateDataPropertyOrThrow(descriptors,
            // key, descriptor): descriptors is ordinary and extensible, so it cannot fail.
            if !matches!(descriptor, Value::Undefined)
                && !self.create_data_property(descriptors, key, descriptor)?
            {
                return Err(self.fault());
            }
        }
        // 5. Return descriptors.
        Ok(Value::Object(descriptors))
    }

    /// `Object.hasOwn ( obj, key )` (`#sec-object.hasown`).
    fn object_has_own(&mut self, args: &[Value]) -> Completion<Value> {
        // 1. Let coerced be ? ToObject(obj).
        let coerced = self.to_object(argument(args, 0))?;
        // 2. Let key be ? ToPropertyKey(key): it may run code, so coerced, which may be new,
        // is kept.
        self.keep(Value::Object(coerced));
        let key = self.to_property_key(argument(args, 1))?;
        // 3. Return ? HasOwnProperty(coerced, key).
        Ok(Value::Boolean(self.has_own_property(coerced, key)?))
    }
}
