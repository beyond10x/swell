//! The Set objects of slice 11 (ECMA-262 24.2, `#sec-set-objects`), written by
//! `story:map-and-set`: `%Set%` (`Set ( [ iterable ] )`, `#sec-set-iterable`) and `get Set [
//! %Symbol.species% ]` (`#sec-get-set-%symbol.species%`); `%Set.prototype%`
//! (`#sec-properties-of-the-set-prototype-object`) with `add`, `clear`, `constructor`, `delete`,
//! `entries`, `forEach`, `has`, `keys` (the same function object as `values`), `get size`,
//! `values`, `[%Symbol.iterator%]` (the same function object as `values`) and
//! `[%Symbol.toStringTag%]`; and the steps of `%SetIteratorPrototype%.next`
//! (`#sec-%setiteratorprototype%.next`), whose prototype `builtins/map/mod.rs` makes beside
//! `%MapIteratorPrototype%`.
//!
//! [[SetData]] is the entry table of the object (`object/collection.rs`), each element a key
//! whose value is `undefined`. The steps a Set shares with a Map (`clear`, `delete`, `has`,
//! `size`, `forEach` and the iterators) are those of `builtins/map/mod.rs`, by the kind of the
//! collection; the loop of the constructor keeps [[Iterator]], [[NextMethod]], the adder and
//! each value on the kept stack until it returns, charges a step per iteration before its work,
//! and closes the iterator on an abrupt completion of the adder (IfAbruptCloseIterator).
//!
//! The set methods (`union` and the six beside it, `story:set-methods`) are the group
//! `set/methods.rs`: the variant [`SetBuiltin::Methods`], its entries in the lists of the
//! prototype's methods, in the order of the clauses.

pub(crate) mod methods;

use crate::builtins::Method;
use crate::builtins::object::argument;
use crate::builtins::set::methods::SetMethodsBuiltin;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};
use crate::object::builtin_function::BuiltinId;
use crate::object::collection::{CollectionKind, IterationKind};
use crate::object::ordinary::OrdinaryData;
use crate::object::property::{Key, Property};
use crate::realm::intrinsics::Making;
use crate::value::{ObjectRef, Value};

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SetBuiltin {
    /// `Set ( [ iterable ] )` (24.2.2.1).
    Constructor,
    /// `get Set [ %Symbol.species% ]` (24.2.3.2).
    Species,
    /// `Set.prototype.add ( value )` (24.2.4.1).
    Add,
    /// `Set.prototype.clear ( )` (24.2.4.2).
    Clear,
    /// `Set.prototype.delete ( value )` (24.2.4.4).
    Delete,
    /// `Set.prototype.entries ( )` (24.2.4.6).
    Entries,
    /// `Set.prototype.forEach ( callbackfn [ , thisArg ] )` (24.2.4.7).
    ForEach,
    /// `Set.prototype.has ( value )` (24.2.4.8).
    Has,
    /// `get Set.prototype.size` (24.2.4.14).
    Size,
    /// `Set.prototype.values ( )` (24.2.4.17), also `keys` and `[%Symbol.iterator%]`.
    Values,
    /// `%SetIteratorPrototype%.next ( )` (24.2.6.2.1).
    IteratorNext,
    /// A builtin of the group `methods.rs`.
    Methods(SetMethodsBuiltin),
}

/// The methods of `%Set.prototype%` after `add`, `clear` and `constructor` and before `keys`,
/// in the order of the clauses.
const PROTOTYPE_METHODS: &[Method<SetBuiltin>] = &[
    ("delete", 1, SetBuiltin::Delete),
    (
        "difference",
        1,
        SetBuiltin::Methods(SetMethodsBuiltin::Difference),
    ),
    ("entries", 0, SetBuiltin::Entries),
    ("forEach", 1, SetBuiltin::ForEach),
    ("has", 1, SetBuiltin::Has),
    (
        "intersection",
        1,
        SetBuiltin::Methods(SetMethodsBuiltin::Intersection),
    ),
    (
        "isDisjointFrom",
        1,
        SetBuiltin::Methods(SetMethodsBuiltin::IsDisjointFrom),
    ),
    (
        "isSubsetOf",
        1,
        SetBuiltin::Methods(SetMethodsBuiltin::IsSubsetOf),
    ),
    (
        "isSupersetOf",
        1,
        SetBuiltin::Methods(SetMethodsBuiltin::IsSupersetOf),
    ),
];

/// The methods of `%Set.prototype%` after `size` and before `values`, in the order of the
/// clauses.
const METHODS_AFTER_SIZE: &[Method<SetBuiltin>] = &[
    (
        "symmetricDifference",
        1,
        SetBuiltin::Methods(SetMethodsBuiltin::SymmetricDifference),
    ),
    ("union", 1, SetBuiltin::Methods(SetMethodsBuiltin::Union)),
];

impl<H: Host> Agent<H> {
    /// `%Set%` and `%Set.prototype%` of the realm being made.
    pub(crate) fn create_set_constructor(
        &mut self,
        making: Making,
    ) -> Completion<(ObjectRef, ObjectRef)> {
        // 24.2.4: "is an ordinary object", "is not a Set", whose [[Prototype]] is
        // %Object.prototype%.
        let prototype =
            self.ordinary_object_create(Some(making.object_prototype), OrdinaryData::Plain)?;
        // 24.2.4.1 `add` and 24.2.4.2 `clear` come before 24.2.4.3 `constructor`.
        for (name, length, id) in [("add", 1, SetBuiltin::Add), ("clear", 0, SetBuiltin::Clear)] {
            self.define_builtin_method(making, prototype, name, length, BuiltinId::Set(id))?;
        }
        // 24.2.3: "length" 0 (24.2.2.1 has one optional parameter); its "prototype" (24.2.3.1)
        // and the prototype's "constructor" (24.2.4.3).
        let constructor = self.create_builtin_constructor(
            making,
            "Set",
            0,
            BuiltinId::Set(SetBuiltin::Constructor),
            prototype,
        )?;
        self.define_species_getter(making, constructor, BuiltinId::Set(SetBuiltin::Species))?;
        self.define_group_methods(making, prototype, PROTOTYPE_METHODS, BuiltinId::Set)?;
        // 24.2.4.12: "The initial value of the "keys" property is %Set.prototype.values%".
        let values =
            self.create_builtin(making, "values", 0, BuiltinId::Set(SetBuiltin::Values))?;
        self.define_builtin_property(prototype, "keys", Value::Object(values))?;
        // 24.2.4.14: an accessor whose set accessor function is undefined.
        self.define_size_getter(making, prototype, BuiltinId::Set(SetBuiltin::Size))?;
        self.define_group_methods(making, prototype, METHODS_AFTER_SIZE, BuiltinId::Set)?;
        self.define_builtin_property(prototype, "values", Value::Object(values))?;
        // 24.2.4.18: "The initial value of the %Symbol.iterator% property is
        // %Set.prototype.values%".
        self.add_property(
            prototype,
            Key::Symbol(making.symbols.iterator),
            Property::data(Value::Object(values), true, false, true),
        )?;
        // 24.2.4.19: "Set", { [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: true }.
        self.define_to_string_tag(making, prototype, "Set")?;
        Ok((constructor, prototype))
    }

    /// The steps of the builtin `id` of this module.
    pub(crate) fn run_set_builtin(
        &mut self,
        id: SetBuiltin,
        _function: ObjectRef,
        this: Value,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        let set = CollectionKind::Set;
        match id {
            SetBuiltin::Constructor => self.set_constructor(argument(args, 0), new_target),
            // 24.2.3.2: "1. Return the this value."
            SetBuiltin::Species => Ok(this),
            SetBuiltin::Add => self.set_add(this, argument(args, 0)),
            SetBuiltin::Clear => self.collection_clear(this, set, "Set.prototype.clear"),
            SetBuiltin::Delete => {
                self.collection_delete(this, set, argument(args, 0), "Set.prototype.delete")
            }
            SetBuiltin::Entries => self.collection_iterator_of(
                this,
                set,
                IterationKind::KeyValue,
                "Set.prototype.entries",
            ),
            SetBuiltin::ForEach => self.collection_for_each(
                this,
                set,
                argument(args, 0),
                argument(args, 1),
                "Set.prototype.forEach",
            ),
            SetBuiltin::Has => {
                self.collection_has(this, set, argument(args, 0), "Set.prototype.has")
            }
            SetBuiltin::Size => self.collection_size(this, set, "get Set.prototype.size"),
            SetBuiltin::Values => {
                self.collection_iterator_of(this, set, IterationKind::Value, "Set.prototype.values")
            }
            SetBuiltin::IteratorNext => self.collection_iterator_next(this, set),
            SetBuiltin::Methods(id) => self.run_set_methods_builtin(id, this, argument(args, 0)),
        }
    }

    /// `Set ( [ iterable ] )` (`#sec-set-iterable`).
    fn set_constructor(
        &mut self,
        iterable: Value,
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        // 1. If NewTarget is undefined, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(self.throw_error(ErrorKind::TypeError, "Constructor Set requires 'new'"));
        };
        // 2. Let set be ? OrdinaryCreateFromConstructor(NewTarget, "%Set.prototype%", «
        // [[SetData]] »). 3. Set set.[[SetData]] to a new empty List.
        let set = self.create_collection(new_target, CollectionKind::Set)?;
        // 4. If iterable is either undefined or null, return set.
        if matches!(iterable, Value::Undefined | Value::Null) {
            return Ok(Value::Object(set));
        }
        // 5. Let adder be ? Get(set, "add"). 6. If IsCallable(adder) is false, throw a
        // TypeError exception. 7. and 8.
        let mark = self.keep(Value::Object(set));
        let filled = self.fill_collection(set, iterable, "add");
        self.release(mark);
        filled?;
        Ok(Value::Object(set))
    }

    /// Steps 7 and 8 of `Set ( [ iterable ] )`, with `adder` (the `add` the constructor read)
    /// and `iterable` held by the caller, which releases what is kept here. A WeakSet's
    /// constructor runs the same loop with its own adder.
    pub(crate) fn add_values_from_iterable(
        &mut self,
        target: ObjectRef,
        iterable: Value,
        adder: Value,
    ) -> Completion<()> {
        // 7. Let iteratorRecord be ? GetIterator(iterable, sync).
        let mut record = self.get_iterator(iterable)?;
        self.keep(Value::Object(record.iterator));
        self.keep(record.next_method);
        // 8. Repeat,
        loop {
            self.charge_steps(1)?;
            // 8.a. Let next be ? IteratorStepValue(iteratorRecord). 8.b. If next is done,
            // return set.
            let Some(next) = self.iterator_step_value(&mut record)? else {
                return Ok(());
            };
            // 8.c. Let status be Completion(Call(adder, set, « next »)). 8.d.
            // IfAbruptCloseIterator(status, iteratorRecord).
            let mark = self.keep(next);
            let added = self.call_value(adder, Value::Object(target), &[next]);
            self.release(mark);
            if let Err(abrupt) = added {
                return Err(self.close_iterator_after(&record, abrupt));
            }
        }
    }

    /// `Set.prototype.add ( value )` (`#sec-set.prototype.add`).
    fn set_add(&mut self, this: Value, value: Value) -> Completion<Value> {
        // 1. Let S be the this value. 2. Perform ? RequireInternalSlot(S, [[SetData]]).
        let set = self.set_data(this, "Set.prototype.add")?;
        // 3. Set value to CanonicalizeKeyedCollectionKey(value). 4. For each element e of
        // S.[[SetData]], if e is not empty and SameValue(e, value) is true, return S. 5.
        // Append value to S.[[SetData]]. An element the table holds keeps its position; its
        // value stays `undefined`.
        self.with_entry_table(set, |agent, table| {
            table.insert(agent, value, Value::Undefined)
        })?;
        // 6. Return S.
        Ok(this)
    }
}
