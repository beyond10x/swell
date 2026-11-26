//! The Reflect object (ECMA-262 28.1, `#sec-reflect-object`), `story:reflect-namespace`:
//! `%Reflect%`, an ordinary object whose [[Prototype]] is `%Object.prototype%`, with its 13
//! functions in the order of their clauses, `Reflect.apply` (`#sec-reflect.apply`) to
//! `Reflect.setPrototypeOf` (`#sec-reflect.setprototypeof`), and `Reflect[%Symbol.toStringTag%]`
//! (`#sec-reflect-%symbol.tostringtag%`). It holds no slot.
//!
//! Each function is a call of one internal method of its target (`object/mod.rs`) after the
//! checks of its clause, so it works on every kind of object the dispatch has. `apply` and
//! `construct` copy their argument list by CreateListFromArrayLike (`builtins/function.rs`), as
//! `Function.prototype.apply` does: a step per element, each element kept on the kept stack and
//! charged to the memory budget, for its kept slot and its slot of the list, before it is kept,
//! and all of it credited when the call returns. `ownKeys` answers the list [[OwnPropertyKeys]]
//! charged, and the list of its values is charged before it is made; CreateArrayFromList
//! charges a step per element before it copies it. A key made by ToPropertyKey is kept while
//! the internal method runs, since its atom is held weakly and a getter, a setter or a step of
//! ToPropertyDescriptor may collect.
//!
//! `apply`, `construct`, `get` and `set` call script (the target, a getter, a setter): each such
//! call is a re-entry of the call path, counted against `reentry_depth` and checked against
//! `frame_depth` there (`docs/design/engine.md`, decision 3, "Depth bounds"). The work of each
//! function is a function of its own, not inlined into the dispatch, whose frame every re-entry
//! through a builtin of this module pays.

use alloc::format;
use alloc::vec::Vec;

use crate::builtins::Method;
use crate::builtins::object::argument;
use crate::host::agent::Agent;
use crate::host::outcome::{Completion, Termination};
use crate::host::{ErrorKind, Host};
use crate::object::builtin_function::BuiltinId;
use crate::object::ordinary::OrdinaryData;
use crate::object::property::Key;
use crate::realm::intrinsics::Making;
use crate::value::{ObjectRef, Value};

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReflectBuiltin {
    /// `Reflect.apply ( target, thisArg, args )` (28.1.1).
    Apply,
    /// `Reflect.construct ( target, args [ , newTarget ] )` (28.1.2).
    Construct,
    /// `Reflect.defineProperty ( target, key, attrs )` (28.1.3).
    DefineProperty,
    /// `Reflect.deleteProperty ( target, key )` (28.1.4).
    DeleteProperty,
    /// `Reflect.get ( target, key [ , receiver ] )` (28.1.5).
    Get,
    /// `Reflect.getOwnPropertyDescriptor ( target, key )` (28.1.6).
    GetOwnPropertyDescriptor,
    /// `Reflect.getPrototypeOf ( target )` (28.1.7).
    GetPrototypeOf,
    /// `Reflect.has ( target, key )` (28.1.8).
    Has,
    /// `Reflect.isExtensible ( target )` (28.1.9).
    IsExtensible,
    /// `Reflect.ownKeys ( target )` (28.1.10).
    OwnKeys,
    /// `Reflect.preventExtensions ( target )` (28.1.11).
    PreventExtensions,
    /// `Reflect.set ( target, key, value [ , receiver ] )` (28.1.12).
    Set,
    /// `Reflect.setPrototypeOf ( target, proto )` (28.1.13).
    SetPrototypeOf,
}

/// The functions of `%Reflect%` in the order of their clauses, each with its "length": the
/// count of its parameters before the first optional one (clause 18), and none a constructor.
pub(crate) const FUNCTIONS: [Method<ReflectBuiltin>; 13] = [
    ("apply", 3, ReflectBuiltin::Apply),
    ("construct", 2, ReflectBuiltin::Construct),
    ("defineProperty", 3, ReflectBuiltin::DefineProperty),
    ("deleteProperty", 2, ReflectBuiltin::DeleteProperty),
    ("get", 2, ReflectBuiltin::Get),
    (
        "getOwnPropertyDescriptor",
        2,
        ReflectBuiltin::GetOwnPropertyDescriptor,
    ),
    ("getPrototypeOf", 1, ReflectBuiltin::GetPrototypeOf),
    ("has", 2, ReflectBuiltin::Has),
    ("isExtensible", 1, ReflectBuiltin::IsExtensible),
    ("ownKeys", 1, ReflectBuiltin::OwnKeys),
    ("preventExtensions", 1, ReflectBuiltin::PreventExtensions),
    ("set", 3, ReflectBuiltin::Set),
    ("setPrototypeOf", 2, ReflectBuiltin::SetPrototypeOf),
];

impl<H: Host> Agent<H> {
    /// `%Reflect%` (`#sec-reflect-object`): an ordinary object of `%Object.prototype%` with the
    /// methods of [`FUNCTIONS`] and its `%Symbol.toStringTag%` "Reflect" ({ [[Writable]]:
    /// false, [[Enumerable]]: false, [[Configurable]]: true }).
    pub(crate) fn create_reflect_object(&mut self, making: Making) -> Completion<ObjectRef> {
        let reflect =
            self.ordinary_object_create(Some(making.object_prototype), OrdinaryData::Plain)?;
        self.define_group_methods(making, reflect, &FUNCTIONS, BuiltinId::Reflect)?;
        self.define_to_string_tag(making, reflect, "Reflect")?;
        Ok(reflect)
    }

    /// The steps of the builtins of this module.
    pub(crate) fn run_reflect_builtin(
        &mut self,
        id: ReflectBuiltin,
        args: &[Value],
    ) -> Completion<Value> {
        match id {
            ReflectBuiltin::Apply => self.reflect_apply(args),
            ReflectBuiltin::Construct => self.reflect_construct(args),
            ReflectBuiltin::DefineProperty => self.reflect_define_property(args),
            ReflectBuiltin::DeleteProperty
            | ReflectBuiltin::GetOwnPropertyDescriptor
            | ReflectBuiltin::Has => self.reflect_key_method(id, args),
            ReflectBuiltin::Get => self.reflect_get(args),
            ReflectBuiltin::GetPrototypeOf
            | ReflectBuiltin::IsExtensible
            | ReflectBuiltin::PreventExtensions => self.reflect_target_method(id, args),
            ReflectBuiltin::OwnKeys => self.reflect_own_keys(args),
            ReflectBuiltin::Set => self.reflect_set(args),
            ReflectBuiltin::SetPrototypeOf => self.reflect_set_prototype_of(args),
        }
    }

    /// Step 1 of the clauses 28.1.3 to 28.1.13: "If target is not an Object, throw a TypeError
    /// exception."
    fn reflect_target(&mut self, args: &[Value], name: &str) -> Completion<ObjectRef> {
        match argument(args, 0) {
            Value::Object(target) => Ok(target),
            _ => Err(self.throw_error(
                ErrorKind::TypeError,
                &format!("Reflect.{name} called on a value that is not an object"),
            )),
        }
    }

    /// `Reflect.apply ( target, thisArg, args )` (`#sec-reflect.apply`). The list is kept in a
    /// room of the kept stack from its first element until the call returns, then released and
    /// credited, as `Function.prototype.apply` keeps it.
    #[inline(never)]
    fn reflect_apply(&mut self, args: &[Value]) -> Completion<Value> {
        // 1. If IsCallable(target) is false, throw a TypeError exception.
        let target = argument(args, 0);
        if !self.is_callable(target)? {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "Reflect.apply called on a target that is not a function",
            ));
        }
        // 2. Let argList be ? CreateListFromArrayLike(args). 3. Perform PrepareForTailCall():
        // no frame to release here. 4. Return ? Call(target, thisArg, argList). The room is
        // ended on every way out, written out so that no closure's frame stands between this
        // call and the callee's re-entry.
        let mark = self.kept.len();
        let mut room = self.kept_room();
        let mut charged = 0;
        let result =
            match self.create_list_from_array_like(argument(args, 2), &mut charged, &mut room) {
                Ok(list) => self.call_value(target, argument(args, 1), &list),
                Err(abrupt) => Err(abrupt),
            };
        self.end_kept_room(mark, &room);
        self.heap.accounting.credit(charged);
        result
    }

    /// `Reflect.construct ( target, args [ , newTarget ] )` (`#sec-reflect.construct`), its
    /// list kept and credited as `Reflect.apply` keeps it.
    #[inline(never)]
    fn reflect_construct(&mut self, args: &[Value]) -> Completion<Value> {
        // 1. If IsConstructor(target) is false, throw a TypeError exception.
        let target = argument(args, 0);
        let Value::Object(function) = target else {
            return Err(self.not_a_constructor("target"));
        };
        if !self.is_constructor(target)? {
            return Err(self.not_a_constructor("target"));
        }
        // 2. If newTarget is not present, set newTarget to target. 3. Else if
        // IsConstructor(newTarget) is false, throw a TypeError exception.
        let new_target = match args.get(2).copied() {
            None => function,
            Some(given) => {
                let Value::Object(new_target) = given else {
                    return Err(self.not_a_constructor("newTarget"));
                };
                if !self.is_constructor(given)? {
                    return Err(self.not_a_constructor("newTarget"));
                }
                new_target
            }
        };
        // 4. Let argList be ? CreateListFromArrayLike(args). 5. Return ? Construct(target,
        // argList, newTarget).
        let mark = self.kept.len();
        let mut room = self.kept_room();
        let mut charged = 0;
        let result =
            match self.create_list_from_array_like(argument(args, 1), &mut charged, &mut room) {
                Ok(list) => self.construct_value(function, &list, Some(new_target)),
                Err(abrupt) => Err(abrupt),
            };
        self.end_kept_room(mark, &room);
        self.heap.accounting.credit(charged);
        result
    }

    /// The TypeError of `Reflect.construct` when `what` is not a constructor.
    fn not_a_constructor(&mut self, what: &str) -> crate::host::outcome::Abrupt {
        self.throw_error(
            ErrorKind::TypeError,
            &format!("Reflect.construct: the {what} is not a constructor"),
        )
    }

    /// Step 2 of the clauses of the functions with a key: "Let propertyKey be ?
    /// ToPropertyKey(key)", the key kept until [`Agent::release`] of the mark it answers.
    fn reflect_key(&mut self, args: &[Value]) -> Completion<(Key, usize)> {
        let key = self.to_property_key(argument(args, 1))?;
        let mark = self.keep_key(key);
        Ok((key, mark))
    }

    /// `Reflect.defineProperty ( target, key, attrs )` (`#sec-reflect.defineproperty`).
    #[inline(never)]
    fn reflect_define_property(&mut self, args: &[Value]) -> Completion<Value> {
        // 1. If target is not an Object, throw a TypeError exception.
        let target = self.reflect_target(args, "defineProperty")?;
        // 2. Let propertyKey be ? ToPropertyKey(key).
        let (key, mark) = self.reflect_key(args)?;
        // 3. Let propertyDesc be ? ToPropertyDescriptor(attrs). 4. Return ?
        // target.[[DefineOwnProperty]](propertyKey, propertyDesc).
        let descriptor = self.to_property_descriptor(argument(args, 2));
        let defined =
            descriptor.and_then(|descriptor| self.define_own_property(target, key, descriptor));
        self.release(mark);
        Ok(Value::Boolean(defined?))
    }

    /// `Reflect.deleteProperty ( target, key )` (`#sec-reflect.deleteproperty`),
    /// `Reflect.getOwnPropertyDescriptor ( target, key )`
    /// (`#sec-reflect.getownpropertydescriptor`) and `Reflect.has ( target, key )`
    /// (`#sec-reflect.has`): step 1, step 2, then the internal method.
    #[inline(never)]
    fn reflect_key_method(&mut self, id: ReflectBuiltin, args: &[Value]) -> Completion<Value> {
        let name = match id {
            ReflectBuiltin::DeleteProperty => "deleteProperty",
            ReflectBuiltin::GetOwnPropertyDescriptor => "getOwnPropertyDescriptor",
            _ => "has",
        };
        // 1. If target is not an Object, throw a TypeError exception.
        let target = self.reflect_target(args, name)?;
        // 2. Let propertyKey be ? ToPropertyKey(key).
        let (key, mark) = self.reflect_key(args)?;
        let answer = match id {
            // 3. Return ? target.[[Delete]](propertyKey).
            ReflectBuiltin::DeleteProperty => self.internal_delete(target, key).map(Value::Boolean),
            // 3. Let propertyDesc be ? target.[[GetOwnProperty]](propertyKey). 4. Return
            // FromPropertyDescriptor(propertyDesc).
            ReflectBuiltin::GetOwnPropertyDescriptor => self
                .get_own_property(target, key)
                .and_then(|property| self.from_property_descriptor(property)),
            // 3. Return ? target.[[HasProperty]](propertyKey).
            _ => self.has_property(target, key).map(Value::Boolean),
        };
        self.release(mark);
        answer
    }

    /// `Reflect.get ( target, key [ , receiver ] )` (`#sec-reflect.get`).
    #[inline(never)]
    fn reflect_get(&mut self, args: &[Value]) -> Completion<Value> {
        // 1. If target is not an Object, throw a TypeError exception.
        let target = self.reflect_target(args, "get")?;
        // 2. Let propertyKey be ? ToPropertyKey(key).
        let (key, mark) = self.reflect_key(args)?;
        // 3. If receiver is not present, then set receiver to target.
        let receiver = args.get(2).copied().unwrap_or(Value::Object(target));
        // 4. Return ? target.[[Get]](propertyKey, receiver).
        let answer = self.internal_get(target, key, receiver);
        self.release(mark);
        answer
    }

    /// `Reflect.set ( target, key, value [ , receiver ] )` (`#sec-reflect.set`).
    #[inline(never)]
    fn reflect_set(&mut self, args: &[Value]) -> Completion<Value> {
        // 1. If target is not an Object, throw a TypeError exception.
        let target = self.reflect_target(args, "set")?;
        // 2. Let propertyKey be ? ToPropertyKey(key).
        let (key, mark) = self.reflect_key(args)?;
        // 3. If receiver is not present, then set receiver to target.
        let receiver = args.get(3).copied().unwrap_or(Value::Object(target));
        // 4. Return ? target.[[Set]](propertyKey, V, receiver).
        let answer = self.internal_set(target, key, argument(args, 2), receiver);
        self.release(mark);
        Ok(Value::Boolean(answer?))
    }

    /// `Reflect.getPrototypeOf ( target )` (`#sec-reflect.getprototypeof`),
    /// `Reflect.isExtensible ( target )` (`#sec-reflect.isextensible`) and
    /// `Reflect.preventExtensions ( target )` (`#sec-reflect.preventextensions`): step 1, then
    /// the internal method.
    #[inline(never)]
    fn reflect_target_method(&mut self, id: ReflectBuiltin, args: &[Value]) -> Completion<Value> {
        let name = match id {
            ReflectBuiltin::GetPrototypeOf => "getPrototypeOf",
            ReflectBuiltin::IsExtensible => "isExtensible",
            _ => "preventExtensions",
        };
        // 1. If target is not an Object, throw a TypeError exception.
        let target = self.reflect_target(args, name)?;
        Ok(match id {
            // 2. Return ? target.[[GetPrototypeOf]]().
            ReflectBuiltin::GetPrototypeOf => self
                .get_prototype_of(target)?
                .map_or(Value::Null, Value::Object),
            // 2. Return ? target.[[IsExtensible]]().
            ReflectBuiltin::IsExtensible => Value::Boolean(self.is_extensible(target)?),
            // 2. Return ? target.[[PreventExtensions]]().
            _ => Value::Boolean(self.prevent_extensions(target)?),
        })
    }

    /// `Reflect.ownKeys ( target )` (`#sec-reflect.ownkeys`).
    #[inline(never)]
    fn reflect_own_keys(&mut self, args: &[Value]) -> Completion<Value> {
        // 1. If target is not an Object, throw a TypeError exception.
        let target = self.reflect_target(args, "ownKeys")?;
        // 2. Let keys be ? target.[[OwnPropertyKeys]](): its steps and its list charged before
        // the list exists. 3. Return CreateArrayFromList(keys).
        let keys = self.own_property_keys(target)?;
        let array = self.array_from_keys(&keys);
        self.drop_key_list(keys);
        array
    }

    /// CreateArrayFromList (`#sec-createarrayfromlist`) of the values of `keys`, Strings and
    /// Symbols: the list of the values is charged to the memory budget before it is reserved
    /// and credited when the array is made; CreateArrayFromList charges a step per element
    /// before it copies it. Nothing between runs code or passes a safe point, so the values
    /// need no root.
    fn array_from_keys(&mut self, keys: &[Key]) -> Completion<Value> {
        let bytes =
            u64::try_from(keys.len().saturating_mul(size_of::<Value>())).unwrap_or(u64::MAX);
        self.charge_bytes(bytes)?;
        let mut values = Vec::new();
        #[expect(
            clippy::disallowed_methods,
            reason = "charged by `charge_bytes` before it is reserved, a `Value` per key"
        )]
        if values.try_reserve_exact(keys.len()).is_err() {
            self.heap.accounting.credit(bytes);
            return Err(self.terminate(Termination::MemoryBudget));
        }
        let mut made = Ok(());
        for &key in keys {
            match self.key_value(key) {
                Ok(value) => values.push(value),
                Err(abrupt) => {
                    made = Err(abrupt);
                    break;
                }
            }
        }
        let array = made.and_then(|()| self.create_array_from_list(&values));
        self.heap.accounting.credit(bytes);
        Ok(Value::Object(array?))
    }

    /// `Reflect.setPrototypeOf ( target, proto )` (`#sec-reflect.setprototypeof`).
    #[inline(never)]
    fn reflect_set_prototype_of(&mut self, args: &[Value]) -> Completion<Value> {
        // 1. If target is not an Object, throw a TypeError exception.
        let target = self.reflect_target(args, "setPrototypeOf")?;
        // 2. If proto is not an Object and proto is not null, throw a TypeError exception.
        let prototype = match argument(args, 1) {
            Value::Object(prototype) => Some(prototype),
            Value::Null => None,
            _ => {
                return Err(self.throw_error(
                    ErrorKind::TypeError,
                    "Reflect.setPrototypeOf: the prototype is neither an object nor null",
                ));
            }
        };
        // 3. Return ? target.[[SetPrototypeOf]](proto).
        Ok(Value::Boolean(self.set_prototype_of(target, prototype)?))
    }
}
