//! The instructions of a Super Reference (`swell_compiler::bytecode`, `SuperBase`,
//! `GetSuperNamed`, `GetSuperIndex`, `PutSuperNamed`, `PutSuperIndex`, `SuperKey` and
//! `DeleteSuper`), each in a function of its own that one arm of `interp/dispatch.rs` calls, so
//! the frame of the instruction loop stays as it was. Written by
//! `story:super-property-and-super-syntax`.
//!
//! A Super Reference is three things on the operand stack: the receiver (GetThisValue, the
//! `this` of the method), the key for a computed one, and the base, which `SuperBase` reads when
//! the reference is made (MakeSuperPropertyReference, `#sec-makesuperpropertyreference`:
//! GetSuperBase, `#sec-getsuperbase`, [[GetPrototypeOf]] of the running method's
//! [[HomeObject]]). GetValue and PutValue (`#sec-getvalue`, `#sec-putvalue`) then take ToObject
//! of the base, a `TypeError` for `null`, before ToPropertyKey of a computed key, and call
//! [[Get]] and [[Set]] of the base with the receiver. The parser admits a SuperProperty only in
//! the code of a method, so the running frame's function has a [[HomeObject]].

use super::dispatch::Flow;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};
use crate::object::ObjectKind;
use crate::value::{ObjectRef, Value};

impl<H: Host> Agent<H> {
    /// `SuperBase`: GetSuperBase of the running method, an Object or `null`.
    #[inline(never)]
    pub(super) fn super_base(&mut self) -> Completion<Flow> {
        let function = self.top_frame()?.function;
        let Some(function) = function else {
            return Err(self.fault());
        };
        let home = match &self.engine_object(function)?.kind {
            ObjectKind::EcmascriptFunction(f) => f.home_object,
            _ => None,
        };
        let Some(home) = home else {
            return Err(self.fault());
        };
        let base = match self.get_prototype_of(home)? {
            Some(prototype) => Value::Object(prototype),
            None => Value::Null,
        };
        self.replace(0, base)
    }

    /// `GetSuperNamed(k)`: `[this, base] -> [value]`.
    #[inline(never)]
    pub(super) fn get_super_named(&mut self, index: u32) -> Completion<Flow> {
        let this = self.peek_value(1)?;
        let base = self.peek_value(0)?;
        let object = self.super_object(base)?;
        let key = self.constant_key(index)?;
        let value = self.internal_get(object, key, this)?;
        self.replace(2, value)
    }

    /// `GetSuperIndex`: `[this, key, base] -> [value]`.
    #[inline(never)]
    pub(super) fn get_super_index(&mut self) -> Completion<Flow> {
        let this = self.peek_value(2)?;
        let key = self.peek_value(1)?;
        let base = self.peek_value(0)?;
        let object = self.super_object(base)?;
        let key = self.to_property_key(key)?;
        let held = self.keep_key(key);
        let value = self.internal_get(object, key, this);
        self.release(held);
        self.replace(3, value?)
    }

    /// `PutSuperNamed(k)`: `[this, base, value] -> [value]`.
    #[inline(never)]
    pub(super) fn put_super_named(&mut self, index: u32, strict: bool) -> Completion<Flow> {
        let this = self.peek_value(2)?;
        let base = self.peek_value(1)?;
        let value = self.peek_value(0)?;
        let object = self.super_object(base)?;
        let key = self.constant_key(index)?;
        let done = self.internal_set(object, key, value, this)?;
        self.super_set_done(done, strict)?;
        self.replace(3, value)
    }

    /// `PutSuperIndex`: `[this, key, base, value] -> [value]`.
    #[inline(never)]
    pub(super) fn put_super_index(&mut self, strict: bool) -> Completion<Flow> {
        let this = self.peek_value(3)?;
        let key = self.peek_value(2)?;
        let base = self.peek_value(1)?;
        let value = self.peek_value(0)?;
        let object = self.super_object(base)?;
        let key = self.to_property_key(key)?;
        let held = self.keep_key(key);
        let done = self.internal_set(object, key, value, this);
        self.release(held);
        self.super_set_done(done?, strict)?;
        self.replace(4, value)
    }

    /// `SuperKey`: `[this, key, base] -> [this, key', base, this, key', base]`, key' the
    /// property key of ToPropertyKey of the key, after ToObject of the base, so that the GetValue
    /// and the PutValue of a compound assignment or an update convert the key once.
    #[inline(never)]
    pub(super) fn super_key(&mut self) -> Completion<Flow> {
        let this = self.peek_value(2)?;
        let key = self.peek_value(1)?;
        let base = self.peek_value(0)?;
        self.super_object(base)?;
        let key = self.to_property_key(key)?;
        let key = self.key_value(key)?;
        self.drop_entries(2)?;
        self.push_value(key)?;
        self.push_value(base)?;
        self.push_value(this)?;
        self.push_value(key)?;
        self.replace(0, base)
    }

    /// `DeleteSuper`: `[this, key] -> []` and a `ReferenceError`
    /// (`#sec-delete-operator-runtime-semantics-evaluation`, step 5.a); the key is not converted.
    #[inline(never)]
    pub(super) fn delete_super(&mut self) -> Completion<Flow> {
        Err(self.throw_error(ErrorKind::ReferenceError, "Cannot delete a super property"))
    }

    /// ToObject of the base of a Super Reference, which GetSuperBase answers as an Object or
    /// `null`.
    fn super_object(&mut self, base: Value) -> Completion<ObjectRef> {
        match base {
            Value::Object(object) => Ok(object),
            _ => Err(self.throw_error(
                ErrorKind::TypeError,
                "Cannot use a super property of a null prototype",
            )),
        }
    }

    /// PutValue's step for a [[Set]] that answered `done`: `false` is a `TypeError` in strict
    /// code.
    fn super_set_done(&mut self, done: bool, strict: bool) -> Completion<()> {
        if !done && strict {
            return Err(self.throw_error(ErrorKind::TypeError, "Cannot assign to the property"));
        }
        Ok(())
    }
}
