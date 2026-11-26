//! Proxy exotic objects (ECMA-262 10.5, `#sec-proxy-object-internal-methods-and-internal-slots`),
//! `story:proxy-objects-and-access-traps`: [[ProxyTarget]] and [[ProxyHandler]], ProxyCreate
//! (`#sec-proxycreate`), ValidateNonRevokedProxy (`#sec-validatenonrevokedproxy`), the
//! revocation, and the 13 internal methods the dispatch of `object/mod.rs` calls for a Proxy.
//!
//! [[Get]], [[Set]], [[HasProperty]], [[Delete]], [[Call]] and [[Construct]] look their trap up
//! on the handler (GetMethod) and, without one, call the target's own method; with one they call
//! it and check its answer by the functions of `object/invariants.rs`, reading each fact of the
//! target at the step of the clause and only under its condition. The seven other methods check
//! for revocation and call the target's own method without reading the handler, until
//! `story:proxy-descriptor-prototype-and-extensibility-traps` gives them their traps.
//!
//! Bounds (`docs/design/engine.md`, decision 3, "Depth bounds"; decision D3 of the decomposition,
//! default A). Every internal method of a Proxy, with or without a trap, is one re-entry for its
//! duration: at its entry it spends a step, then the activations plus the methods of proxies
//! already running (`Interp::proxy_nesting`) plus this one passing `reentry_depth` throw the
//! RangeError of "Depth bounds" before the handler is read; then it counts itself in
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

use super::invariants::{self, Broken, Next};
use super::property::{Descriptor, Key, Property};
use super::{KeyList, ObjectKind};
use crate::heap::collect::Mark;
use crate::host::agent::Agent;
use crate::host::outcome::{Abrupt, Completion};
use crate::host::{ErrorKind, Host};
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

/// The traps of this part, by the name GetMethod looks up on the handler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Trap {
    Get,
    Set,
    Has,
    DeleteProperty,
    Apply,
    Construct,
}

impl Trap {
    /// The name of the trap.
    fn name(self) -> &'static str {
        match self {
            Trap::Get => "get",
            Trap::Set => "set",
            Trap::Has => "has",
            Trap::DeleteProperty => "deleteProperty",
            Trap::Apply => "apply",
            Trap::Construct => "construct",
        }
    }
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
    /// of "Depth bounds" before the handler is read; then the method counts itself.
    fn enter_proxy(&mut self) -> Completion<()> {
        self.running()?;
        self.charge_steps(1)?;
        let activations = u32::try_from(self.activations.len()).unwrap_or(u32::MAX);
        let reentries = activations
            .saturating_add(self.interp.proxy_nesting)
            .saturating_add(1);
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

    /// One of the seven internal methods without a trap of this part: ValidateNonRevokedProxy,
    /// then `forward` with the target, the handler not read.
    fn proxy_forward<T>(
        &mut self,
        proxy: ObjectRef,
        forward: impl FnOnce(&mut Self, ObjectRef) -> Completion<T>,
    ) -> Completion<T> {
        self.in_proxy_method(|agent| {
            let (target, _) = agent.kept_proxy_slots(proxy)?;
            forward(agent, target)
        })
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

    /// [[GetPrototypeOf]] (10.5.1) of this part: the target's.
    pub(crate) fn proxy_get_prototype_of(
        &mut self,
        proxy: ObjectRef,
    ) -> Completion<Option<ObjectRef>> {
        self.proxy_forward(proxy, |agent, target| agent.get_prototype_of(target))
    }

    /// [[SetPrototypeOf]] (10.5.2) of this part: the target's.
    pub(crate) fn proxy_set_prototype_of(
        &mut self,
        proxy: ObjectRef,
        prototype: Option<ObjectRef>,
    ) -> Completion<bool> {
        self.proxy_forward(proxy, |agent, target| {
            agent.set_prototype_of(target, prototype)
        })
    }

    /// [[IsExtensible]] (10.5.3) of this part: the target's.
    pub(crate) fn proxy_is_extensible(&mut self, proxy: ObjectRef) -> Completion<bool> {
        self.proxy_forward(proxy, |agent, target| agent.is_extensible(target))
    }

    /// [[PreventExtensions]] (10.5.4) of this part: the target's.
    pub(crate) fn proxy_prevent_extensions(&mut self, proxy: ObjectRef) -> Completion<bool> {
        self.proxy_forward(proxy, |agent, target| agent.prevent_extensions(target))
    }

    /// [[GetOwnProperty]] (10.5.5) of this part: the target's.
    pub(crate) fn proxy_get_own_property(
        &mut self,
        proxy: ObjectRef,
        key: Key,
    ) -> Completion<Option<Property>> {
        self.proxy_forward(proxy, |agent, target| agent.get_own_property(target, key))
    }

    /// [[DefineOwnProperty]] (10.5.6) of this part: the target's.
    pub(crate) fn proxy_define_own_property(
        &mut self,
        proxy: ObjectRef,
        key: Key,
        descriptor: Descriptor,
    ) -> Completion<bool> {
        self.proxy_forward(proxy, |agent, target| {
            agent.define_own_property(target, key, descriptor)
        })
    }

    /// [[OwnPropertyKeys]] (10.5.11) of this part: the target's list, with its charge.
    pub(crate) fn proxy_own_property_keys(&mut self, proxy: ObjectRef) -> Completion<KeyList> {
        self.proxy_forward(proxy, |agent, target| agent.own_property_keys(target))
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
