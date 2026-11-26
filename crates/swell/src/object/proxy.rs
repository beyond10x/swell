//! Proxy exotic objects (ECMA-262 10.5, `#sec-proxy-object-internal-methods-and-internal-slots`),
//! `story:proxy-objects-and-access-traps`: [[ProxyTarget]] and [[ProxyHandler]], ProxyCreate
//! (`#sec-proxycreate`), ValidateNonRevokedProxy (`#sec-validatenonrevokedproxy`), the
//! revocation, and the 13 internal methods the dispatch of `object/mod.rs` calls for a Proxy.
//!
//! Each of the 13 methods looks its trap up on the handler (GetMethod) and, without one, calls
//! the target's own method; with one it calls it and checks its answer by the functions of
//! `object/invariants.rs`, reading each fact of the target at the step of the clause and only
//! under its condition. `story:proxy-descriptor-prototype-and-extensibility-traps` gave the seven
//! methods [[GetPrototypeOf]], [[SetPrototypeOf]], [[IsExtensible]], [[PreventExtensions]],
//! [[GetOwnProperty]], [[DefineOwnProperty]] and [[OwnPropertyKeys]] their traps (10.5.1 to
//! 10.5.6 and 10.5.11); the TypeError of a broken invariant names the trap, the key and the
//! invariant's number (I1 to I21). The answer of `ownKeys` is made a list by
//! CreateListFromArrayLike with a step per element before it is read, each element kept and its
//! slot charged; each check of the clause spends a step per key it visits; the map of the
//! answer's keys and the list of the target's non-configurable keys are charged before they are
//! made and credited when the method returns.
//!
//! Bounds (`docs/design/engine.md`, decision 3, "Depth bounds"; decision D3 of the decomposition,
//! default A). Every internal method of a Proxy, with or without a trap, is one re-entry for its
//! duration: at its entry it spends a step, then the activations plus the methods of proxies
//! already running (`Interp::proxy_nesting`) plus this one passing `reentry_depth` throw the
//! RangeError of "Depth bounds" before the handler is read (a builtin's activation innermost is
//! this one, as it is for code the builtin calls); then it counts itself in
//! `proxy_nesting` until it returns. `Agent::enter_code` adds `proxy_nesting` to the re-entries it
//! compares, so a chain of proxies of any length, forwarding with no script, ends in that
//! RangeError and not in a Rust stack overflow. A trap call is the re-entry of the method that
//! calls it, not a second one: the method hands its count to the call while the trap runs. What a
//! method holds across a call into code (the target, the handler, the key, the receiver, the
//! trap and its answer, the arguments and their Array) is kept on the kept stack until it
//! returns; the arguments of [[Call]] and [[Construct]] in room of the kept stack charged before
//! it grows. The record of a Proxy, and the Arrays of `apply` and `construct`, are charged when
//! they are made (`Agent::allocate_object`, CreateArrayFromList).
//!
//! Kind site of the collector: [`ProxyObject::trace`] reports the target and the handler.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::builtin_function::KeptRoom;
use super::invariants::{self, Broken, Next};
use super::ordinary::OrdinaryData;
use super::property::{Descriptor, Key, Property};
use super::{KeyList, ObjectKind};
use crate::heap::collect::Mark;
use crate::heap::map::KeyedMap;
use crate::host::agent::Agent;
use crate::host::outcome::{Abrupt, Completion, Termination};
use crate::host::{ErrorKind, Host};
use crate::realm::intrinsics::Intrinsic;
use crate::script::vec_bytes;
use crate::value::{ObjectRef, Value};

/// The slots of a Proxy exotic object (Table 31): [[ProxyTarget]] and [[ProxyHandler]], both
/// `None` once revoked (`#sec-proxy.revocable`), and whether it has [[Call]] and [[Construct]],
/// fixed by ProxyCreate from its target (steps 7 and 8).
pub(crate) struct ProxyObject {
    /// [[ProxyTarget]].
    target: Option<ObjectRef>,
    /// [[ProxyHandler]].
    handler: Option<ObjectRef>,
    call: bool,
    construct: bool,
}

impl ProxyObject {
    /// Kind site of the collector: the target and the handler, while not revoked.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/proxy.rs ProxyObject::trace");
        if let Some(target) = self.target {
            mark.object(target);
        }
        if let Some(handler) = self.handler {
            mark.object(handler);
        }
    }

    /// Whether it has a [[Call]] (ProxyCreate, step 7).
    pub(crate) fn has_call(&self) -> bool {
        self.call
    }

    /// Whether it has a [[Construct]] (ProxyCreate, step 8).
    pub(crate) fn has_construct(&self) -> bool {
        self.construct
    }

    /// A Proxy whose target and handler are `named`, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test(named: ObjectRef) -> ProxyObject {
        ProxyObject {
            target: Some(named),
            handler: Some(named),
            call: false,
            construct: false,
        }
    }
}

/// The 13 traps, by the name GetMethod looks up on the handler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Trap {
    GetPrototypeOf,
    SetPrototypeOf,
    IsExtensible,
    PreventExtensions,
    GetOwnPropertyDescriptor,
    DefineProperty,
    Get,
    Set,
    Has,
    DeleteProperty,
    OwnKeys,
    Apply,
    Construct,
}

impl Trap {
    /// The name of the trap.
    fn name(self) -> &'static str {
        match self {
            Trap::GetPrototypeOf => "getPrototypeOf",
            Trap::SetPrototypeOf => "setPrototypeOf",
            Trap::IsExtensible => "isExtensible",
            Trap::PreventExtensions => "preventExtensions",
            Trap::GetOwnPropertyDescriptor => "getOwnPropertyDescriptor",
            Trap::DefineProperty => "defineProperty",
            Trap::Get => "get",
            Trap::Set => "set",
            Trap::Has => "has",
            Trap::DeleteProperty => "deleteProperty",
            Trap::OwnKeys => "ownKeys",
            Trap::Apply => "apply",
            Trap::Construct => "construct",
        }
    }
}

/// The map of the keys of an `ownKeys` answer (10.5.11, steps 9 and 18): every key of the answer,
/// then uncheckedResultKeys, from which each key of the target is removed (a removal never grows
/// the table), and the bytes
/// its table was charged, credited when the method returns.
struct SeenKeys {
    keys: KeyedMap<Key, ()>,
    bytes: u64,
}

impl<H: Host> Agent<H> {
    /// ProxyCreate ( target, handler ) (`#sec-proxycreate`): a TypeError when either is not an
    /// Object; [[Call]] and [[Construct]] when the target has them. The record is charged before
    /// it is stored; `target` and `handler` are the caller's, which holds them.
    pub(crate) fn proxy_create(&mut self, target: Value, handler: Value) -> Completion<ObjectRef> {
        // 1. If target is not an Object, throw a TypeError exception.
        let Value::Object(target) = target else {
            return Err(
                self.throw_error(ErrorKind::TypeError, "Proxy: the target is not an object")
            );
        };
        // 2. If handler is not an Object, throw a TypeError exception.
        let Value::Object(handler) = handler else {
            return Err(
                self.throw_error(ErrorKind::TypeError, "Proxy: the handler is not an object")
            );
        };
        // 3 to 9. MakeBasicObject, with [[Call]] if IsCallable(target) and [[Construct]] if
        // IsConstructor(target), and both slots set.
        let call = self.has_call(target)?;
        let construct = self.has_construct(target)?;
        self.allocate_object(
            None,
            ObjectKind::Proxy(ProxyObject {
                target: Some(target),
                handler: Some(handler),
                call,
                construct,
            }),
        )
    }

    /// The steps of revokerClosure (`#sec-proxy.revocable`, step 2.e and 2.f): the Proxy's
    /// [[ProxyTarget]] and [[ProxyHandler]] set to null, so that it holds neither.
    pub(crate) fn revoke_proxy(&mut self, proxy: ObjectRef) -> Completion<()> {
        if let ObjectKind::Proxy(slots) = &mut self.engine_object_mut(proxy)?.kind {
            slots.target = None;
            slots.handler = None;
            return Ok(());
        }
        Err(self.fault())
    }

    /// ValidateNonRevokedProxy (`#sec-validatenonrevokedproxy`) and [[ProxyTarget]], for IsArray
    /// and GetFunctionRealm, which go on with the target in their own loop: a revoked Proxy is a
    /// TypeError. Nothing is kept.
    pub(crate) fn proxy_target(&mut self, proxy: ObjectRef) -> Completion<ObjectRef> {
        Ok(self.proxy_slots(proxy)?.0)
    }

    /// ValidateNonRevokedProxy, then [[ProxyTarget]] and [[ProxyHandler]] (steps 1 to 3 of each
    /// internal method).
    fn proxy_slots(&mut self, proxy: ObjectRef) -> Completion<(ObjectRef, ObjectRef)> {
        let slots = match &self.engine_object(proxy)?.kind {
            ObjectKind::Proxy(slots) => Some((slots.target, slots.handler)),
            _ => None,
        };
        match slots {
            Some((Some(target), Some(handler))) => Ok((target, handler)),
            // 1. If proxy.[[ProxyTarget]] is null, throw a TypeError exception.
            Some(_) => Err(self.throw_error(ErrorKind::TypeError, "the proxy has been revoked")),
            None => Err(self.fault()),
        }
    }

    /// The slots of a running internal method, kept until it returns.
    fn kept_proxy_slots(&mut self, proxy: ObjectRef) -> Completion<(ObjectRef, ObjectRef)> {
        let (target, handler) = self.proxy_slots(proxy)?;
        self.keep(Value::Object(target));
        self.keep(Value::Object(handler));
        Ok((target, handler))
    }

    /// The entry of an internal method of a Proxy (the module's documentation, "Bounds"): the
    /// sticky termination, a step, then the re-entry count, whose bound throws the RangeError
    /// of "Depth bounds" before the handler is read; then the method counts itself. The count is
    /// the one `Agent::enter_code` compares: when a builtin's activation is innermost (above the
    /// running loop's), that activation is the re-entry this method makes, as it is for code the
    /// builtin calls; otherwise the method adds one.
    fn enter_proxy(&mut self) -> Completion<()> {
        self.running()?;
        self.charge_steps(1)?;
        let depth = self.activations.len();
        let own = u32::from(depth <= self.interp.running.unwrap_or(0));
        let activations = u32::try_from(depth).unwrap_or(u32::MAX);
        let reentries = activations
            .saturating_add(self.interp.proxy_nesting)
            .saturating_add(own);
        if reentries > self.limits.reentry_depth {
            return Err(self.throw_error(ErrorKind::RangeError, "reentry_depth exceeded"));
        }
        self.interp.proxy_nesting = self.interp.proxy_nesting.saturating_add(1);
        Ok(())
    }

    /// Run `steps`, an internal method of a Proxy, between [`Agent::enter_proxy`] and its exit:
    /// what it kept is released and its count dropped on every way out.
    fn in_proxy_method<T>(
        &mut self,
        steps: impl FnOnce(&mut Self) -> Completion<T>,
    ) -> Completion<T> {
        self.enter_proxy()?;
        let mark = self.kept.len();
        let answer = steps(self);
        self.release(mark);
        self.interp.proxy_nesting = self.interp.proxy_nesting.saturating_sub(1);
        answer
    }

    /// `? GetMethod(handler, name)` (step 5 of each internal method with a trap), the trap kept.
    fn proxy_trap(&mut self, handler: ObjectRef, trap: Trap) -> Completion<Option<Value>> {
        let key = self.name_key(trap.name())?;
        let found = self.get_method(Value::Object(handler), key)?;
        if let Some(trap) = found {
            self.keep(trap);
        }
        Ok(found)
    }

    /// `? Call(trap, handler, args)`: the arguments kept, the method's count handed to the call
    /// while the trap runs (a trap call is the re-entry of the method that calls it), the answer
    /// kept.
    fn call_trap(&mut self, trap: Value, handler: ObjectRef, args: &[Value]) -> Completion<Value> {
        for &arg in args {
            self.keep(arg);
        }
        self.interp.proxy_nesting = self.interp.proxy_nesting.saturating_sub(1);
        let answer = self.call_value(trap, Value::Object(handler), args);
        self.interp.proxy_nesting = self.interp.proxy_nesting.saturating_add(1);
        if let Ok(value) = answer {
            self.keep(value);
        }
        answer
    }

    /// The TypeError of an invariant a trap broke, naming the trap and the key.
    fn broken_invariant(&mut self, trap: Trap, key: Option<Key>, broken: Broken) -> Abrupt {
        let message = match key {
            Some(Key::Symbol(_)) => format!(
                "proxy trap {} broke an invariant for a Symbol key: {}",
                trap.name(),
                broken.text()
            ),
            Some(key) => {
                let name: String = self.key_text(key);
                format!(
                    "proxy trap {} broke an invariant for the key \"{name}\": {}",
                    trap.name(),
                    broken.text()
                )
            }
            None => format!(
                "proxy trap {} broke an invariant: {}",
                trap.name(),
                broken.text()
            ),
        };
        self.throw_error(ErrorKind::TypeError, &message)
    }

    /// The values a property holds, kept until the method returns.
    fn keep_property(&mut self, property: Option<Property>) {
        if let Some(property) = property {
            self.keep_descriptor(&invariants::descriptor_of(property));
        }
    }

    /// The values a descriptor holds, kept until the method returns.
    fn keep_descriptor(&mut self, descriptor: &Descriptor) {
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
    }

    /// FromPropertyDescriptor (`#sec-frompropertydescriptor`) of `descriptor`, which may lack
    /// fields: an ordinary object of `%Object.prototype%` of the current realm with a data
    /// property for each field it has, in the order "value", "writable", "get", "set",
    /// "enumerable", "configurable". The object is new and extensible, so each
    /// CreateDataPropertyOrThrow succeeds and runs no code.
    fn descriptor_object(&mut self, descriptor: &Descriptor) -> Completion<Value> {
        let prototype = self.current_intrinsic(Intrinsic::ObjectPrototype)?;
        let object = self.ordinary_object_create(Some(prototype), OrdinaryData::Plain)?;
        let accessor =
            |function: Option<ObjectRef>| function.map_or(Value::Undefined, Value::Object);
        let fields: [(&str, Option<Value>); 6] = [
            ("value", descriptor.value),
            ("writable", descriptor.writable.map(Value::Boolean)),
            ("get", descriptor.get.map(accessor)),
            ("set", descriptor.set.map(accessor)),
            ("enumerable", descriptor.enumerable.map(Value::Boolean)),
            ("configurable", descriptor.configurable.map(Value::Boolean)),
        ];
        for (name, value) in fields {
            let Some(value) = value else {
                continue;
            };
            let key = self.name_key(name)?;
            if !self.create_data_property(object, key, value)? {
                return Err(self.fault());
            }
        }
        Ok(Value::Object(object))
    }

    /// [[GetPrototypeOf]] (`#sec-proxy-object-internal-methods-and-internal-slots-getprototypeof`).
    pub(crate) fn proxy_get_prototype_of(
        &mut self,
        proxy: ObjectRef,
    ) -> Completion<Option<ObjectRef>> {
        self.in_proxy_method(|agent| agent.proxy_get_prototype_of_steps(proxy))
    }

    /// The steps of [[GetPrototypeOf]] after the entry.
    #[inline(never)]
    fn proxy_get_prototype_of_steps(&mut self, proxy: ObjectRef) -> Completion<Option<ObjectRef>> {
        // 1 to 4.
        let (target, handler) = self.kept_proxy_slots(proxy)?;
        // 5. Let trap be ? GetMethod(handler, "getPrototypeOf"). 6. If trap is undefined,
        // return ? target.[[GetPrototypeOf]]().
        let Some(trap) = self.proxy_trap(handler, Trap::GetPrototypeOf)? else {
            return self.get_prototype_of(target);
        };
        // 7. Let handlerProto be ? Call(trap, handler, « target »).
        let answer = self.call_trap(trap, handler, &[Value::Object(target)])?;
        // 8. If handlerProto is neither an Object nor null, throw a TypeError (I1).
        let answer = invariants::get_prototype_of_answer(answer)
            .map_err(|broken| self.broken_invariant(Trap::GetPrototypeOf, None, broken))?;
        // 9. Let extensibleTarget be ? IsExtensible(target). 10. If it is true, return
        // handlerProto.
        let extensible = self.is_extensible(target)?;
        if invariants::get_prototype_of_after_extensible(extensible) == Next::TrapResult {
            return Ok(answer);
        }
        // 11. Let targetProto be ? target.[[GetPrototypeOf]](). 12. If SameValue(handlerProto,
        // targetProto) is false, throw a TypeError (I2). 13. Return handlerProto.
        let prototype = self.get_prototype_of(target)?;
        invariants::get_prototype_of_after_prototype(answer, prototype)
            .map_err(|broken| self.broken_invariant(Trap::GetPrototypeOf, None, broken))?;
        Ok(answer)
    }

    /// [[SetPrototypeOf]] (`#sec-proxy-object-internal-methods-and-internal-slots-setprototypeof-v`).
    pub(crate) fn proxy_set_prototype_of(
        &mut self,
        proxy: ObjectRef,
        prototype: Option<ObjectRef>,
    ) -> Completion<bool> {
        self.in_proxy_method(|agent| agent.proxy_set_prototype_of_steps(proxy, prototype))
    }

    /// The steps of [[SetPrototypeOf]] after the entry.
    #[inline(never)]
    fn proxy_set_prototype_of_steps(
        &mut self,
        proxy: ObjectRef,
        prototype: Option<ObjectRef>,
    ) -> Completion<bool> {
        // 1 to 4.
        let (target, handler) = self.kept_proxy_slots(proxy)?;
        let value = prototype.map_or(Value::Null, Value::Object);
        self.keep(value);
        // 5. Let trap be ? GetMethod(handler, "setPrototypeOf"). 6. If trap is undefined,
        // return ? target.[[SetPrototypeOf]](V).
        let Some(trap) = self.proxy_trap(handler, Trap::SetPrototypeOf)? else {
            return self.set_prototype_of(target, prototype);
        };
        // 7. Let booleanTrapResult be ToBoolean(? Call(trap, handler, « target, V »)).
        let answer = self.call_trap(trap, handler, &[Value::Object(target), value])?;
        let answer = self.to_boolean(answer)?;
        // 8. If booleanTrapResult is false, return false.
        if let Next::Answer(answer) = invariants::set_prototype_of_after_trap(answer) {
            return Ok(answer);
        }
        // 9. Let extensibleTarget be ? IsExtensible(target). 10. If it is true, return true.
        let extensible = self.is_extensible(target)?;
        if let Next::Answer(answer) = invariants::set_prototype_of_after_extensible(extensible) {
            return Ok(answer);
        }
        // 11. Let targetProto be ? target.[[GetPrototypeOf]](). 12. If SameValue(V,
        // targetProto) is false, throw a TypeError (I3). 13. Return true.
        let current = self.get_prototype_of(target)?;
        invariants::set_prototype_of_after_prototype(prototype, current)
            .map_err(|broken| self.broken_invariant(Trap::SetPrototypeOf, None, broken))
    }

    /// [[IsExtensible]] (`#sec-proxy-object-internal-methods-and-internal-slots-isextensible`).
    pub(crate) fn proxy_is_extensible(&mut self, proxy: ObjectRef) -> Completion<bool> {
        self.in_proxy_method(|agent| agent.proxy_is_extensible_steps(proxy))
    }

    /// The steps of [[IsExtensible]] after the entry.
    #[inline(never)]
    fn proxy_is_extensible_steps(&mut self, proxy: ObjectRef) -> Completion<bool> {
        // 1 to 4.
        let (target, handler) = self.kept_proxy_slots(proxy)?;
        // 5. Let trap be ? GetMethod(handler, "isExtensible"). 6. If trap is undefined, return
        // ? IsExtensible(target).
        let Some(trap) = self.proxy_trap(handler, Trap::IsExtensible)? else {
            return self.is_extensible(target);
        };
        // 7. Let booleanTrapResult be ToBoolean(? Call(trap, handler, « target »)).
        let answer = self.call_trap(trap, handler, &[Value::Object(target)])?;
        let answer = self.to_boolean(answer)?;
        // 8. Let targetResult be ? IsExtensible(target). 9. If booleanTrapResult is not
        // targetResult, throw a TypeError (I4). 10. Return booleanTrapResult.
        let extensible = self.is_extensible(target)?;
        invariants::is_extensible_after_target(answer, extensible)
            .map_err(|broken| self.broken_invariant(Trap::IsExtensible, None, broken))
    }

    /// [[PreventExtensions]] (`#sec-proxy-object-internal-methods-and-internal-slots-preventextensions`).
    pub(crate) fn proxy_prevent_extensions(&mut self, proxy: ObjectRef) -> Completion<bool> {
        self.in_proxy_method(|agent| agent.proxy_prevent_extensions_steps(proxy))
    }

    /// The steps of [[PreventExtensions]] after the entry.
    #[inline(never)]
    fn proxy_prevent_extensions_steps(&mut self, proxy: ObjectRef) -> Completion<bool> {
        // 1 to 4.
        let (target, handler) = self.kept_proxy_slots(proxy)?;
        // 5. Let trap be ? GetMethod(handler, "preventExtensions"). 6. If trap is undefined,
        // return ? target.[[PreventExtensions]]().
        let Some(trap) = self.proxy_trap(handler, Trap::PreventExtensions)? else {
            return self.prevent_extensions(target);
        };
        // 7. Let booleanTrapResult be ToBoolean(? Call(trap, handler, « target »)).
        let answer = self.call_trap(trap, handler, &[Value::Object(target)])?;
        let answer = self.to_boolean(answer)?;
        // 9. A false booleanTrapResult is returned.
        if let Next::Answer(answer) = invariants::prevent_extensions_after_trap(answer) {
            return Ok(answer);
        }
        // 8.a. Let extensibleTarget be ? IsExtensible(target). 8.b. If it is true, throw a
        // TypeError (I5). 9. Return booleanTrapResult.
        let extensible = self.is_extensible(target)?;
        invariants::prevent_extensions_after_extensible(extensible)
            .map_err(|broken| self.broken_invariant(Trap::PreventExtensions, None, broken))
    }

    /// [[GetOwnProperty]] (`#sec-proxy-object-internal-methods-and-internal-slots-getownproperty-p`).
    pub(crate) fn proxy_get_own_property(
        &mut self,
        proxy: ObjectRef,
        key: Key,
    ) -> Completion<Option<Property>> {
        self.in_proxy_method(|agent| agent.proxy_get_own_property_steps(proxy, key))
    }

    /// The steps of [[GetOwnProperty]] after the entry.
    #[inline(never)]
    fn proxy_get_own_property_steps(
        &mut self,
        proxy: ObjectRef,
        key: Key,
    ) -> Completion<Option<Property>> {
        // 1 to 4.
        let (target, handler) = self.kept_proxy_slots(proxy)?;
        self.keep_key(key);
        // 5. Let trap be ? GetMethod(handler, "getOwnPropertyDescriptor"). 6. If trap is
        // undefined, return ? target.[[GetOwnProperty]](P).
        let Some(trap) = self.proxy_trap(handler, Trap::GetOwnPropertyDescriptor)? else {
            return self.get_own_property(target, key);
        };
        // 7. Let trapResultObj be ? Call(trap, handler, « target, P »).
        let name = self.key_value(key)?;
        let answer = self.call_trap(trap, handler, &[Value::Object(target), name])?;
        // 8. If trapResultObj is neither an Object nor undefined, throw a TypeError (I6).
        let broken = |agent: &mut Self, which: Broken| {
            agent.broken_invariant(Trap::GetOwnPropertyDescriptor, Some(key), which)
        };
        let answer = match invariants::get_own_property_answer(answer) {
            Ok(answer) => answer,
            Err(error) => return Err(broken(self, error)),
        };
        // 9. Let targetDesc be ? target.[[GetOwnProperty]](P).
        let current = self.get_own_property(target, key)?;
        self.keep_property(current);
        // 10. If trapResultObj is undefined: none answers undefined; a non-configurable one
        // throws (I7); else IsExtensible(target), false throws (I8); return undefined.
        let Some(answer) = answer else {
            return match invariants::get_own_property_undefined_after_descriptor(current) {
                Ok(Next::ReadExtensible) => {
                    let extensible = self.is_extensible(target)?;
                    match invariants::get_own_property_undefined_after_extensible(extensible) {
                        Ok(()) => Ok(None),
                        Err(error) => Err(broken(self, error)),
                    }
                }
                Ok(_) => Ok(None),
                Err(error) => Err(broken(self, error)),
            };
        };
        // 11. Let extensibleTarget be ? IsExtensible(target).
        let extensible = self.is_extensible(target)?;
        // 12. Let resultDesc be ? ToPropertyDescriptor(trapResultObj). 13.
        // CompletePropertyDescriptor(resultDesc).
        let descriptor = self.to_property_descriptor(Value::Object(answer))?;
        self.keep_descriptor(&descriptor);
        let result = invariants::complete_property_descriptor(&descriptor);
        // 14 to 16. The checks I9, I10 and I11. 17. Return resultDesc.
        match invariants::get_own_property_result(self, extensible, result, current)? {
            Ok(()) => Ok(Some(result)),
            Err(error) => Err(broken(self, error)),
        }
    }

    /// [[DefineOwnProperty]] (`#sec-proxy-object-internal-methods-and-internal-slots-defineownproperty-p-desc`).
    pub(crate) fn proxy_define_own_property(
        &mut self,
        proxy: ObjectRef,
        key: Key,
        descriptor: Descriptor,
    ) -> Completion<bool> {
        self.in_proxy_method(|agent| agent.proxy_define_own_property_steps(proxy, key, descriptor))
    }

    /// The steps of [[DefineOwnProperty]] after the entry.
    #[inline(never)]
    fn proxy_define_own_property_steps(
        &mut self,
        proxy: ObjectRef,
        key: Key,
        descriptor: Descriptor,
    ) -> Completion<bool> {
        // 1 to 4.
        let (target, handler) = self.kept_proxy_slots(proxy)?;
        self.keep_key(key);
        self.keep_descriptor(&descriptor);
        // 5. Let trap be ? GetMethod(handler, "defineProperty"). 6. If trap is undefined,
        // return ? target.[[DefineOwnProperty]](P, Desc).
        let Some(trap) = self.proxy_trap(handler, Trap::DefineProperty)? else {
            return self.define_own_property(target, key, descriptor);
        };
        // 7. Let descObj be FromPropertyDescriptor(Desc). 8. Let booleanTrapResult be
        // ToBoolean(? Call(trap, handler, « target, P, descObj »)).
        let object = self.descriptor_object(&descriptor)?;
        self.keep(object);
        let name = self.key_value(key)?;
        let answer = self.call_trap(trap, handler, &[Value::Object(target), name, object])?;
        let answer = self.to_boolean(answer)?;
        // 9. If booleanTrapResult is false, return false.
        if let Next::Answer(answer) = invariants::define_own_property_after_trap(answer) {
            return Ok(answer);
        }
        // 10. Let targetDesc be ? target.[[GetOwnProperty]](P). 11. Let extensibleTarget be ?
        // IsExtensible(target).
        let current = self.get_own_property(target, key)?;
        self.keep_property(current);
        let extensible = self.is_extensible(target)?;
        // 12 to 15. The checks I12 to I16. 16. Return true.
        match invariants::define_own_property(self, extensible, descriptor, current)? {
            Ok(()) => Ok(true),
            Err(broken) => Err(self.broken_invariant(Trap::DefineProperty, Some(key), broken)),
        }
    }

    /// [[OwnPropertyKeys]] (`#sec-proxy-object-internal-methods-and-internal-slots-ownpropertykeys`).
    /// The trap's answer is a list made by CreateListFromArrayLike, a step per element before it
    /// is read, each element kept and charged; each check of the clause spends a step per key it
    /// visits, before it reads it; the map of the answer's keys is charged before it is made, and
    /// every list but the one answered is credited when the method returns. The list answered
    /// is charged as every [[OwnPropertyKeys]] list is ([`Agent::new_key_list`]).
    pub(crate) fn proxy_own_property_keys(&mut self, proxy: ObjectRef) -> Completion<KeyList> {
        self.in_proxy_method(|agent| agent.proxy_own_keys_steps(proxy))
    }

    /// The steps of [[OwnPropertyKeys]] after the entry, to the trap's answer.
    #[inline(never)]
    fn proxy_own_keys_steps(&mut self, proxy: ObjectRef) -> Completion<KeyList> {
        // 1 to 4.
        let (target, handler) = self.kept_proxy_slots(proxy)?;
        // 5. Let trap be ? GetMethod(handler, "ownKeys"). 6. If trap is undefined, return ?
        // target.[[OwnPropertyKeys]]().
        let Some(trap) = self.proxy_trap(handler, Trap::OwnKeys)? else {
            return self.own_property_keys(target);
        };
        // 7. Let trapResultArray be ? Call(trap, handler, « target »).
        let array = self.call_trap(trap, handler, &[Value::Object(target)])?;
        self.with_kept_room(|agent, room| agent.proxy_own_keys_list(target, array, room))
    }

    /// Step 8 of [[OwnPropertyKeys]]: trapResult, CreateListFromArrayLike(trapResultArray,
    /// property-key), an element that is not a property key throwing (I17); then the steps after
    /// it. The list's slots are credited on every way out.
    #[inline(never)]
    fn proxy_own_keys_list(
        &mut self,
        target: ObjectRef,
        array: Value,
        room: &mut KeptRoom,
    ) -> Completion<KeyList> {
        let mut charged = 0;
        let mut refuse = |agent: &mut Self, element: Value| {
            invariants::own_keys_element(element)
                .err()
                .map(|broken| agent.broken_invariant(Trap::OwnKeys, None, broken))
        };
        let listed = self.create_list_of_property_keys(array, &mut refuse, &mut charged, room);
        let answer = match listed {
            Ok(list) => {
                let mut seen = SeenKeys {
                    keys: KeyedMap::with_hasher(self.heap.hasher),
                    bytes: 0,
                };
                let answer = self.proxy_own_keys_seen(target, &list, room, &mut seen);
                self.heap.accounting.credit(seen.bytes);
                answer
            }
            Err(abrupt) => Err(abrupt),
        };
        self.heap.accounting.credit(charged);
        answer
    }

    /// Step 9 of [[OwnPropertyKeys]] and the steps after it, over the trap's list `list`: the
    /// map of its keys is charged for all of them before it is made, the list answered is made
    /// ([`Agent::new_key_list`]) and credited here on an abrupt completion.
    fn proxy_own_keys_seen(
        &mut self,
        target: ObjectRef,
        list: &[Value],
        room: &mut KeptRoom,
        seen: &mut SeenKeys,
    ) -> Completion<KeyList> {
        let wanted = KeyedMap::<Key, ()>::table_bytes_for(list.len());
        self.charge_bytes(wanted)?;
        seen.bytes = wanted;
        if seen.keys.reserve(list.len()).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        let table = seen.keys.table_bytes();
        if table > seen.bytes {
            // The map took more than its layout says: charged now.
            self.charge_bytes(table - seen.bytes)?;
            seen.bytes = table;
        }
        let mut keys = self.new_key_list(list.len())?;
        match self.proxy_own_keys_check(target, list, room, seen, &mut keys.keys) {
            Ok(()) => Ok(keys),
            Err(abrupt) => {
                self.drop_key_list(keys);
                Err(abrupt)
            }
        }
    }

    /// Steps 9 to 11 of [[OwnPropertyKeys]]: each element of `list` made a key, a step before
    /// each, kept and pushed to `keys`, a key listed twice throwing (I18); then IsExtensible
    /// and the target's own keys, credited on every way out after the checks against them.
    fn proxy_own_keys_check(
        &mut self,
        target: ObjectRef,
        list: &[Value],
        room: &mut KeptRoom,
        seen: &mut SeenKeys,
        keys: &mut Vec<Key>,
    ) -> Completion<()> {
        self.reserve_kept(room, list.len())?;
        // 9. If trapResult contains any duplicate entries, throw a TypeError (I18).
        for &element in list {
            self.charge_steps(1)?;
            let key = self.to_property_key(element)?;
            self.keep_key(key);
            let before = match seen.keys.insert(key, ()) {
                Ok(before) => before.is_some(),
                Err(_) => return Err(self.terminate(Termination::MemoryBudget)),
            };
            if let Err(broken) = invariants::own_keys_listed_before(before) {
                return Err(self.broken_invariant(Trap::OwnKeys, Some(key), broken));
            }
            keys.push(key);
        }
        // 10. Let extensibleTarget be ? IsExtensible(target).
        let extensible = self.is_extensible(target)?;
        // 11. Let targetKeys be ? target.[[OwnPropertyKeys]]().
        let target_keys = self.own_property_keys(target)?;
        let checked =
            self.proxy_own_keys_against_target(target, extensible, &target_keys, room, seen, keys);
        self.drop_key_list(target_keys);
        checked
    }

    /// Steps 12 to 22 of [[OwnPropertyKeys]] over the target's keys `target_keys`, each kept:
    /// whether each is non-configurable, read by the target's [[GetOwnProperty]] in their order
    /// in a list charged before it is reserved and credited on every way out; then the checks
    /// I19, I20 and I21, a step per key each visits.
    #[inline(never)]
    fn proxy_own_keys_against_target(
        &mut self,
        target: ObjectRef,
        extensible: bool,
        target_keys: &[Key],
        room: &mut KeptRoom,
        seen: &mut SeenKeys,
        keys: &[Key],
    ) -> Completion<()> {
        self.reserve_kept(room, target_keys.len())?;
        for &key in target_keys {
            self.keep_key(key);
        }
        let bytes = vec_bytes::<bool>(target_keys.len());
        self.charge_bytes(bytes)?;
        let mut fixed: Vec<bool> = Vec::new();
        #[expect(
            clippy::disallowed_methods,
            reason = "charged by `charge_bytes` before it is reserved, a `bool` per key of the target"
        )]
        if fixed.try_reserve_exact(target_keys.len()).is_err() {
            self.heap.accounting.credit(bytes);
            return Err(self.terminate(Termination::MemoryBudget));
        }
        let checked =
            self.proxy_own_keys_fixed(target, extensible, target_keys, seen, keys, &mut fixed);
        drop(fixed);
        self.heap.accounting.credit(bytes);
        checked
    }

    /// Steps 12 to 22 of [[OwnPropertyKeys]] with the room for `fixed`, whether each key of the
    /// target is non-configurable.
    fn proxy_own_keys_fixed(
        &mut self,
        target: ObjectRef,
        extensible: bool,
        target_keys: &[Key],
        seen: &mut SeenKeys,
        keys: &[Key],
        fixed: &mut Vec<bool>,
    ) -> Completion<()> {
        // 12 to 16. For each key of targetKeys: Let desc be ? target.[[GetOwnProperty]](key);
        // a non-configurable one goes to targetNonconfigurableKeys, others to
        // targetConfigurableKeys.
        for &key in target_keys {
            self.charge_steps(1)?;
            let descriptor = self.get_own_property(target, key)?;
            fixed.push(descriptor.is_some_and(|property| !property.configurable));
        }
        // 17. An extensible target with no non-configurable key returns trapResult.
        let any_fixed = fixed.contains(&true);
        if invariants::own_keys_after_target(extensible, any_fixed) == Next::TrapResult {
            return Ok(());
        }
        // 18. Let uncheckedResultKeys be the elements of trapResult: the keys of `seen`. 19.
        // For each non-configurable key: not among them throws (I19); remove it.
        for (&key, _) in target_keys
            .iter()
            .zip(fixed.iter())
            .filter(|(_, fixed)| **fixed)
        {
            self.proxy_own_keys_remove(key, false, seen)?;
        }
        // 20. If extensibleTarget is true, return trapResult.
        if invariants::own_keys_after_non_configurable(extensible) == Next::TrapResult {
            return Ok(());
        }
        // 21. For each configurable key: not among them throws (I20); remove it.
        for (&key, _) in target_keys
            .iter()
            .zip(fixed.iter())
            .filter(|(_, fixed)| !**fixed)
        {
            self.proxy_own_keys_remove(key, true, seen)?;
        }
        // 22. If uncheckedResultKeys is not empty, throw a TypeError (I21), naming the first
        // key of trapResult left, found a step per key visited.
        let unchecked = seen.keys.len();
        if let Err(broken) = invariants::own_keys_rest(unchecked) {
            let mut left = None;
            for &key in keys {
                self.charge_steps(1)?;
                if seen.keys.get(&key).is_some() {
                    left = Some(key);
                    break;
                }
            }
            return Err(self.broken_invariant(Trap::OwnKeys, left, broken));
        }
        // 23. Return trapResult.
        Ok(())
    }

    /// Steps 19 and 21 of [[OwnPropertyKeys]] for the target's key `key`, a step before it is
    /// looked up: unless it is among uncheckedResultKeys, a TypeError (I19 for a
    /// non-configurable key, I20 for a configurable one); it is removed from them.
    fn proxy_own_keys_remove(
        &mut self,
        key: Key,
        configurable: bool,
        seen: &mut SeenKeys,
    ) -> Completion<()> {
        self.charge_steps(1)?;
        let unchecked = seen.keys.remove(&key).is_some();
        invariants::own_keys_target_key(unchecked, configurable)
            .map_err(|broken| self.broken_invariant(Trap::OwnKeys, Some(key), broken))
    }

    /// [[HasProperty]] (`#sec-proxy-object-internal-methods-and-internal-slots-hasproperty-p`).
    pub(crate) fn proxy_has_property(&mut self, proxy: ObjectRef, key: Key) -> Completion<bool> {
        self.in_proxy_method(|agent| agent.proxy_has_steps(proxy, key))
    }

    /// The steps of [[HasProperty]] after the entry.
    #[inline(never)]
    fn proxy_has_steps(&mut self, proxy: ObjectRef, key: Key) -> Completion<bool> {
        // 1 to 4.
        let (target, handler) = self.kept_proxy_slots(proxy)?;
        self.keep_key(key);
        // 5. Let trap be ? GetMethod(handler, "has"). 6. If trap is undefined, then return ?
        // target.[[HasProperty]](P).
        let Some(trap) = self.proxy_trap(handler, Trap::Has)? else {
            return self.has_property(target, key);
        };
        // 7. Let booleanTrapResult be ToBoolean(? Call(trap, handler, « target, P »)).
        let name = self.key_value(key)?;
        let answer = self.call_trap(trap, handler, &[Value::Object(target), name])?;
        let answer = self.to_boolean(answer)?;
        // 8. If booleanTrapResult is false: 9. Return booleanTrapResult.
        if let Next::Answer(answer) = invariants::has_after_trap(answer) {
            return Ok(answer);
        }
        // 8.a. Let targetDesc be ? target.[[GetOwnProperty]](P).
        let descriptor = self.get_own_property(target, key)?;
        // 8.b.i. A non-configurable one throws; 8.b.ii. Let extensibleTarget be ?
        // IsExtensible(target); 8.b.iii. If it is false, throw.
        match invariants::has_after_descriptor(descriptor) {
            Ok(Next::Answer(answer)) => Ok(answer),
            Ok(_) => {
                let extensible = self.is_extensible(target)?;
                invariants::has_after_extensible(extensible)
                    .map_err(|broken| self.broken_invariant(Trap::Has, Some(key), broken))
            }
            Err(broken) => Err(self.broken_invariant(Trap::Has, Some(key), broken)),
        }
    }

    /// [[Get]] (`#sec-proxy-object-internal-methods-and-internal-slots-get-p-receiver`).
    pub(crate) fn proxy_get(
        &mut self,
        proxy: ObjectRef,
        key: Key,
        receiver: Value,
    ) -> Completion<Value> {
        self.in_proxy_method(|agent| agent.proxy_get_steps(proxy, key, receiver))
    }

    /// The steps of [[Get]] after the entry.
    #[inline(never)]
    fn proxy_get_steps(
        &mut self,
        proxy: ObjectRef,
        key: Key,
        receiver: Value,
    ) -> Completion<Value> {
        // 1 to 4.
        let (target, handler) = self.kept_proxy_slots(proxy)?;
        self.keep_key(key);
        self.keep(receiver);
        // 5. Let trap be ? GetMethod(handler, "get"). 6. If trap is undefined, then return ?
        // target.[[Get]](P, Receiver).
        let Some(trap) = self.proxy_trap(handler, Trap::Get)? else {
            return self.internal_get(target, key, receiver);
        };
        // 7. Let trapResult be ? Call(trap, handler, « target, P, Receiver »).
        let name = self.key_value(key)?;
        let answer = self.call_trap(trap, handler, &[Value::Object(target), name, receiver])?;
        // 8. Let targetDesc be ? target.[[GetOwnProperty]](P).
        let descriptor = self.get_own_property(target, key)?;
        // 9. The checks of 9.a.i and 9.b.i. 10. Return trapResult.
        match invariants::get(self, descriptor, answer)? {
            Ok(()) => Ok(answer),
            Err(broken) => Err(self.broken_invariant(Trap::Get, Some(key), broken)),
        }
    }

    /// [[Set]] (`#sec-proxy-object-internal-methods-and-internal-slots-set-p-v-receiver`).
    pub(crate) fn proxy_set(
        &mut self,
        proxy: ObjectRef,
        key: Key,
        value: Value,
        receiver: Value,
    ) -> Completion<bool> {
        self.in_proxy_method(|agent| agent.proxy_set_steps(proxy, key, value, receiver))
    }

    /// The steps of [[Set]] after the entry.
    #[inline(never)]
    fn proxy_set_steps(
        &mut self,
        proxy: ObjectRef,
        key: Key,
        value: Value,
        receiver: Value,
    ) -> Completion<bool> {
        // 1 to 4.
        let (target, handler) = self.kept_proxy_slots(proxy)?;
        self.keep_key(key);
        self.keep(value);
        self.keep(receiver);
        // 5. Let trap be ? GetMethod(handler, "set"). 6. If trap is undefined, then return ?
        // target.[[Set]](P, V, Receiver).
        let Some(trap) = self.proxy_trap(handler, Trap::Set)? else {
            return self.internal_set(target, key, value, receiver);
        };
        // 7. Let booleanTrapResult be ToBoolean(? Call(trap, handler, « target, P, V, Receiver
        // »)).
        let name = self.key_value(key)?;
        let answer = self.call_trap(
            trap,
            handler,
            &[Value::Object(target), name, value, receiver],
        )?;
        let answer = self.to_boolean(answer)?;
        // 8. If booleanTrapResult is false, return false.
        if let Next::Answer(answer) = invariants::set_after_trap(answer) {
            return Ok(answer);
        }
        // 9. Let targetDesc be ? target.[[GetOwnProperty]](P).
        let descriptor = self.get_own_property(target, key)?;
        // 10. The checks of 10.a.i and 10.b.i. 11. Return true.
        match invariants::set(self, descriptor, value)? {
            Ok(()) => Ok(true),
            Err(broken) => Err(self.broken_invariant(Trap::Set, Some(key), broken)),
        }
    }

    /// [[Delete]] (`#sec-proxy-object-internal-methods-and-internal-slots-delete-p`).
    pub(crate) fn proxy_delete(&mut self, proxy: ObjectRef, key: Key) -> Completion<bool> {
        self.in_proxy_method(|agent| agent.proxy_delete_steps(proxy, key))
    }

    /// The steps of [[Delete]] after the entry.
    #[inline(never)]
    fn proxy_delete_steps(&mut self, proxy: ObjectRef, key: Key) -> Completion<bool> {
        // 1 to 4.
        let (target, handler) = self.kept_proxy_slots(proxy)?;
        self.keep_key(key);
        // 5. Let trap be ? GetMethod(handler, "deleteProperty"). 6. If trap is undefined, then
        // return ? target.[[Delete]](P).
        let Some(trap) = self.proxy_trap(handler, Trap::DeleteProperty)? else {
            return self.internal_delete(target, key);
        };
        // 7. Let booleanTrapResult be ToBoolean(? Call(trap, handler, « target, P »)).
        let name = self.key_value(key)?;
        let answer = self.call_trap(trap, handler, &[Value::Object(target), name])?;
        let answer = self.to_boolean(answer)?;
        // 8. If booleanTrapResult is false, return false.
        if let Next::Answer(answer) = invariants::delete_after_trap(answer) {
            return Ok(answer);
        }
        // 9. Let targetDesc be ? target.[[GetOwnProperty]](P).
        let descriptor = self.get_own_property(target, key)?;
        // 10. If targetDesc is undefined, return true. 11. A non-configurable one throws. 12.
        // Let extensibleTarget be ? IsExtensible(target). 13. If it is false, throw.
        match invariants::delete_after_descriptor(descriptor) {
            Ok(Next::Answer(answer)) => Ok(answer),
            Ok(_) => {
                let extensible = self.is_extensible(target)?;
                invariants::delete_after_extensible(extensible).map_err(|broken| {
                    self.broken_invariant(Trap::DeleteProperty, Some(key), broken)
                })
            }
            Err(broken) => Err(self.broken_invariant(Trap::DeleteProperty, Some(key), broken)),
        }
    }

    /// The arguments of [[Call]] or [[Construct]] kept across the method, in room of the kept
    /// stack charged before it grows, then `steps`; the room is ended on every way out.
    fn with_kept_arguments<T>(
        &mut self,
        args: &[Value],
        steps: impl FnOnce(&mut Self) -> Completion<T>,
    ) -> Completion<T> {
        self.with_kept_room(|agent, room| {
            agent.reserve_kept(room, args.len())?;
            for &arg in args {
                agent.keep(arg);
            }
            steps(agent)
        })
    }

    /// [[Call]] (`#sec-proxy-object-internal-methods-and-internal-slots-call-thisargument-argumentslist`).
    /// The caller has checked that the Proxy has a [[Call]].
    pub(crate) fn proxy_call(
        &mut self,
        proxy: ObjectRef,
        this: Value,
        args: &[Value],
    ) -> Completion<Value> {
        self.in_proxy_method(|agent| {
            agent.with_kept_arguments(args, |agent| agent.proxy_call_steps(proxy, this, args))
        })
    }

    /// The steps of [[Call]] after the entry.
    #[inline(never)]
    fn proxy_call_steps(
        &mut self,
        proxy: ObjectRef,
        this: Value,
        args: &[Value],
    ) -> Completion<Value> {
        // 1 to 4.
        let (target, handler) = self.kept_proxy_slots(proxy)?;
        self.keep(this);
        // 5. Let trap be ? GetMethod(handler, "apply"). 6. If trap is undefined, then return ?
        // Call(target, thisArgument, argumentsList).
        let Some(trap) = self.proxy_trap(handler, Trap::Apply)? else {
            return self.call_value(Value::Object(target), this, args);
        };
        // 7. Let argArray be CreateArrayFromList(argumentsList).
        let array = Value::Object(self.create_array_from_list(args)?);
        // 8. Return ? Call(trap, handler, « target, thisArgument, argArray »).
        self.call_trap(trap, handler, &[Value::Object(target), this, array])
    }

    /// [[Construct]] (`#sec-proxy-object-internal-methods-and-internal-slots-construct-argumentslist-newtarget`).
    /// The caller has checked that the Proxy and `new_target` are constructors.
    pub(crate) fn proxy_construct(
        &mut self,
        proxy: ObjectRef,
        args: &[Value],
        new_target: ObjectRef,
    ) -> Completion<Value> {
        self.in_proxy_method(|agent| {
            agent.with_kept_arguments(args, |agent| {
                agent.proxy_construct_steps(proxy, args, new_target)
            })
        })
    }

    /// The steps of [[Construct]] after the entry.
    #[inline(never)]
    fn proxy_construct_steps(
        &mut self,
        proxy: ObjectRef,
        args: &[Value],
        new_target: ObjectRef,
    ) -> Completion<Value> {
        // 1 to 5. Assert: IsConstructor(target), which ProxyCreate fixed.
        let (target, handler) = self.kept_proxy_slots(proxy)?;
        self.keep(Value::Object(new_target));
        // 6. Let trap be ? GetMethod(handler, "construct"). 7. If trap is undefined, then return
        // ? Construct(target, argumentsList, newTarget).
        let Some(trap) = self.proxy_trap(handler, Trap::Construct)? else {
            return self.internal_construct(target, args, new_target);
        };
        // 8. Let argArray be CreateArrayFromList(argumentsList).
        let array = Value::Object(self.create_array_from_list(args)?);
        // 9. Let newObj be ? Call(trap, handler, « target, argArray, newTarget »).
        let answer = self.call_trap(
            trap,
            handler,
            &[Value::Object(target), array, Value::Object(new_target)],
        )?;
        // 10. If newObj is not an Object, throw a TypeError exception. 11. Return newObj.
        match invariants::construct(answer) {
            Ok(object) => Ok(Value::Object(object)),
            Err(broken) => Err(self.broken_invariant(Trap::Construct, None, broken)),
        }
    }
}
