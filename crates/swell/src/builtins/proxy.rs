//! The Proxy constructor (ECMA-262 28.2, `#sec-proxy-objects`),
//! `story:proxy-objects-and-access-traps`: `%Proxy%` (`#sec-proxy-constructor`), a constructor
//! with no "prototype" property whose [[Prototype]] is `%Function.prototype%`, `length` 2, and
//! `Proxy.revocable` (`#sec-proxy.revocable`) with the revoke function it makes. The Proxy
//! exotic object itself and its internal methods are `object/proxy.rs`.
//!
//! Memory: the Proxy record (ProxyCreate), the `{ proxy, revoke }` object and the revoke function
//! with the box of its slot ([`Slots::Revocable`]) are each charged before they exist. What is
//! made is kept on the kept stack until the next is made from it.

use crate::builtins::object::argument;
use crate::heap::arena::Handle;
use crate::host::ErrorKind;
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::object::builtin_function::{Behaviour, BuiltinId, Slots};
use crate::object::ordinary::OrdinaryData;
use crate::realm::intrinsics::{Intrinsic, Making};
use crate::value::{ObjectRef, Value};

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProxyBuiltin {
    /// `Proxy ( target, handler )` (28.2.1.1).
    Constructor,
    /// `Proxy.revocable ( target, handler )` (28.2.2.1).
    Revocable,
    /// revokerClosure of `Proxy.revocable` (28.2.2.1, step 2), with the slot
    /// [`Slots::Revocable`].
    Revoke,
}

impl<H: Host> Agent<H> {
    /// `%Proxy%` (`#sec-proxy-constructor`, `#sec-properties-of-the-proxy-constructor`): a
    /// constructor named "Proxy" of `length` 2 with no "prototype" property, and its method
    /// `revocable` of `length` 2.
    pub(crate) fn create_proxy_constructor(&mut self, making: Making) -> Completion<ObjectRef> {
        let proxy = self.create_builtin_function(
            Behaviour::Builtin(BuiltinId::Proxy(ProxyBuiltin::Constructor)),
            2,
            "Proxy",
            making.realm,
            Some(making.function_prototype),
            true,
        )?;
        self.define_builtin_method(
            making,
            proxy,
            "revocable",
            2,
            BuiltinId::Proxy(ProxyBuiltin::Revocable),
        )?;
        Ok(proxy)
    }

    /// The steps of the builtins of this module.
    pub(crate) fn run_proxy_builtin(
        &mut self,
        id: ProxyBuiltin,
        function: ObjectRef,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        match id {
            ProxyBuiltin::Constructor => self.proxy_constructor(args, new_target),
            ProxyBuiltin::Revocable => self.proxy_revocable(args),
            ProxyBuiltin::Revoke => self.proxy_revoke(function),
        }
    }

    /// `Proxy ( target, handler )` (`#sec-proxy-target-handler`).
    #[inline(never)]
    fn proxy_constructor(
        &mut self,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        // 1. If NewTarget is undefined, throw a TypeError exception.
        if new_target.is_none() {
            return Err(self.throw_error(ErrorKind::TypeError, "Constructor Proxy requires 'new'"));
        }
        // 2. Return ? ProxyCreate(target, handler).
        let proxy = self.proxy_create(argument(args, 0), argument(args, 1))?;
        Ok(Value::Object(proxy))
    }

    /// `Proxy.revocable ( target, handler )` (`#sec-proxy.revocable`).
    #[inline(never)]
    fn proxy_revocable(&mut self, args: &[Value]) -> Completion<Value> {
        // 1. Let proxy be ? ProxyCreate(target, handler).
        let proxy = self.proxy_create(argument(args, 0), argument(args, 1))?;
        let mark = self.keep(Value::Object(proxy));
        let answer = self.revocable_result(proxy);
        self.release(mark);
        answer.map(Value::Object)
    }

    /// Steps 2 to 8 of `Proxy.revocable`, `proxy` kept by the caller.
    fn revocable_result(&mut self, proxy: ObjectRef) -> Completion<ObjectRef> {
        // 2 to 4. Let revoker be CreateBuiltinFunction(revokerClosure, 0, "", «
        // [[RevocableProxy]] »), its [[RevocableProxy]] proxy.
        let realm: Handle = self.current_realm()?;
        let revoker = self.create_builtin_closure(
            BuiltinId::Proxy(ProxyBuiltin::Revoke),
            Slots::Revocable { proxy: Some(proxy) },
            0,
            realm,
        )?;
        self.keep(Value::Object(revoker));
        // 5. Let result be OrdinaryObjectCreate(%Object.prototype%).
        let prototype = self.current_intrinsic(Intrinsic::ObjectPrototype)?;
        let result = self.ordinary_object_create(Some(prototype), OrdinaryData::Plain)?;
        self.keep(Value::Object(result));
        // 6. Perform ! CreateDataPropertyOrThrow(result, "proxy", proxy).
        let key = self.name_key("proxy")?;
        self.create_data_property_or_throw(result, key, Value::Object(proxy))?;
        // 7. Perform ! CreateDataPropertyOrThrow(result, "revoke", revoker).
        let key = self.name_key("revoke")?;
        self.create_data_property_or_throw(result, key, Value::Object(revoker))?;
        // 8. Return result.
        Ok(result)
    }

    /// revokerClosure (`#sec-proxy.revocable`, step 2): the first call revokes the Proxy and
    /// drops it from the slot; every call answers `undefined`.
    #[inline(never)]
    fn proxy_revoke(&mut self, function: ObjectRef) -> Completion<Value> {
        // a. Let F be the active function object. b. Let p be F.[[RevocableProxy]].
        let revoked = match self.closure_slots_mut(function)? {
            // d. Set F.[[RevocableProxy]] to null.
            Slots::Revocable { proxy } => proxy.take(),
            Slots::Resolving { .. }
            | Slots::Executor { .. }
            | Slots::Await { .. }
            | Slots::AsyncFromSync(_) => return Err(self.fault()),
        };
        // c. If p is null, return undefined.
        if let Some(proxy) = revoked {
            // e to g. Set p.[[ProxyTarget]] and p.[[ProxyHandler]] to null.
            self.revoke_proxy(proxy)?;
        }
        // h. Return undefined.
        Ok(Value::Undefined)
    }
}
