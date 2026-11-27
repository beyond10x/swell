//! The WeakSet objects (ECMA-262 24.4, `#sec-weakset-objects`), written by
//! `story:weak-map-and-weak-set`: `%WeakSet%` (`WeakSet ( [ iterable ] )`,
//! `#sec-weakset-iterable`, whose loop over the iterable with the adder `add` is the one of
//! `builtins/set/mod.rs`) and `%WeakSet.prototype%` (`#sec-properties-of-the-weakset-prototype-object`)
//! with `add`, `constructor`, `delete`, `has` and `[%Symbol.toStringTag%]`. An element is an
//! object or a Symbol that CanBeHeldWeakly (`#sec-canbeheldweakly`); [[WeakSetData]] is the
//! table of `object/weak_collection.rs`, each entry's value `undefined`.

use crate::builtins::object::argument;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};
use crate::object::builtin_function::BuiltinId;
use crate::object::ordinary::OrdinaryData;
use crate::object::weak_collection::WeakKind;
use crate::realm::intrinsics::Making;
use crate::value::{ObjectRef, Value};

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WeakSetBuiltin {
    /// `WeakSet ( [ iterable ] )` (24.4.1.1).
    Constructor,
    /// `WeakSet.prototype.add ( value )` (24.4.3.1).
    Add,
    /// `WeakSet.prototype.delete ( value )` (24.4.3.3).
    Delete,
    /// `WeakSet.prototype.has ( value )` (24.4.3.4).
    Has,
}

impl<H: Host> Agent<H> {
    /// `%WeakSet%` and `%WeakSet.prototype%` of the realm being made.
    pub(crate) fn create_weak_set_constructor(
        &mut self,
        making: Making,
    ) -> Completion<(ObjectRef, ObjectRef)> {
        // 24.4.3: "is an ordinary object", whose [[Prototype]] is %Object.prototype%.
        let prototype =
            self.ordinary_object_create(Some(making.object_prototype), OrdinaryData::Plain)?;
        // 24.4.3.1 `add` comes before 24.4.3.2 `constructor`.
        self.define_builtin_method(
            making,
            prototype,
            "add",
            1,
            BuiltinId::WeakSet(WeakSetBuiltin::Add),
        )?;
        // 24.4.2: "length" 0 (24.4.1.1 has one optional parameter); its "prototype" (24.4.2.1)
        // and the prototype's "constructor" (24.4.3.2).
        let constructor = self.create_builtin_constructor(
            making,
            "WeakSet",
            0,
            BuiltinId::WeakSet(WeakSetBuiltin::Constructor),
            prototype,
        )?;
        for (name, id) in [
            ("delete", WeakSetBuiltin::Delete),
            ("has", WeakSetBuiltin::Has),
        ] {
            self.define_builtin_method(making, prototype, name, 1, BuiltinId::WeakSet(id))?;
        }
        // 24.4.3.5: "WeakSet", { [[Writable]]: false, [[Enumerable]]: false,
        // [[Configurable]]: true }.
        self.define_to_string_tag(making, prototype, "WeakSet")?;
        Ok((constructor, prototype))
    }

    /// The steps of the builtin `id` of this module.
    pub(crate) fn run_weak_set_builtin(
        &mut self,
        id: WeakSetBuiltin,
        this: Value,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        let value = argument(args, 0);
        match id {
            WeakSetBuiltin::Constructor => self.weak_set_constructor(value, new_target),
            WeakSetBuiltin::Add => self.weak_set_add(this, value),
            WeakSetBuiltin::Delete => self.weak_set_delete(this, value),
            WeakSetBuiltin::Has => self.weak_set_has(this, value),
        }
    }

    /// `WeakSet ( [ iterable ] )` (`#sec-weakset-iterable`).
    fn weak_set_constructor(
        &mut self,
        iterable: Value,
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        // 1. If NewTarget is undefined, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(
                self.throw_error(ErrorKind::TypeError, "Constructor WeakSet requires 'new'")
            );
        };
        // 2. Let set be ? OrdinaryCreateFromConstructor(NewTarget, "%WeakSet.prototype%", «
        // [[WeakSetData]] »). 3. Set set.[[WeakSetData]] to a new empty List.
        let set = self.create_weak_collection(new_target, WeakKind::Set)?;
        // 4. If iterable is either undefined or null, return set.
        if matches!(iterable, Value::Undefined | Value::Null) {
            return Ok(Value::Object(set));
        }
        // 5. Let adder be ? Get(set, "add"). 6. If IsCallable(adder) is false, throw a
        // TypeError exception. 7. to 8. The loop over the iterable, calling adder with each
        // value.
        let mark = self.keep(Value::Object(set));
        let filled = self.fill_collection(set, iterable, "add");
        self.release(mark);
        filled?;
        Ok(Value::Object(set))
    }

    /// `WeakSet.prototype.add ( value )` (`#sec-weakset.prototype.add`).
    fn weak_set_add(&mut self, this: Value, value: Value) -> Completion<Value> {
        // 1. Let weakSet be the this value. 2. Perform ? RequireInternalSlot(weakSet,
        // [[WeakSetData]]).
        let set = self.weak_collection_of(this, WeakKind::Set, "WeakSet.prototype.add")?;
        // 3. If CanBeHeldWeakly(value) is false, throw a TypeError exception.
        let Some(key) = self.weak_key(value)? else {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "a WeakSet value must be an object or a Symbol not in the registry",
            ));
        };
        // 4. If value is an element already, return weakSet. 5. Append value to
        // weakSet.[[WeakSetData]]. 6. Return weakSet.
        self.weak_insert(set, key, Value::Undefined)?;
        Ok(this)
    }

    /// `WeakSet.prototype.delete ( value )` (`#sec-weakset.prototype.delete`).
    fn weak_set_delete(&mut self, this: Value, value: Value) -> Completion<Value> {
        let set = self.weak_collection_of(this, WeakKind::Set, "WeakSet.prototype.delete")?;
        // 3. If CanBeHeldWeakly(value) is false, return false.
        let Some(key) = self.weak_key(value)? else {
            return Ok(Value::Boolean(false));
        };
        Ok(Value::Boolean(self.weak_delete(set, key)?))
    }

    /// `WeakSet.prototype.has ( value )` (`#sec-weakset.prototype.has`).
    fn weak_set_has(&mut self, this: Value, value: Value) -> Completion<Value> {
        let set = self.weak_collection_of(this, WeakKind::Set, "WeakSet.prototype.has")?;
        // 3. If CanBeHeldWeakly(value) is false, return false.
        let Some(key) = self.weak_key(value)? else {
            return Ok(Value::Boolean(false));
        };
        Ok(Value::Boolean(self.weak_get(set, key)?.is_some()))
    }
}
