//! The instructions of private names (`swell_compiler::bytecode`, after `PutIndex`), each in a
//! function of its own that one arm of `interp/dispatch.rs` calls, so the frame of the
//! instruction loop stays as it was (`docs/design/engine.md`, "Private names", items 3 and 5).
//! Written by `story:private-names-and-private-methods`.
//!
//! A Private Name lives in a slot of the private record of its class, which the class's code
//! pushes inside the class scope and fills with `NewPrivateName`, one new Private Name per
//! private identifier at each evaluation of the class. `GetPrivate` and `SetPrivate` read it at
//! the hops and slot the compiler resolved and run GetValue and PutValue of the private
//! reference (`#sec-getvalue`, `#sec-putvalue`, step 3): the base converted by ToObject, which
//! throws a `TypeError` for `undefined` and `null` and gives any other primitive a wrapper that
//! has no private element, so a primitive base is a `TypeError` either way and no wrapper is
//! made; then PrivateGet or PrivateSet (`object/private_names.rs`), a getter or setter called
//! as a property's is, with the object kept on the operand stack while it runs.
//! `AddPrivateMethod` runs PrivateMethodOrAccessorAdd of the function kept in the private
//! record. A slot that holds no Private Name where one is read is a logic error of the engine
//! (section 13): the agent faults.

use alloc::format;

use super::dispatch::Flow;
use super::frame::SlotValue;
use crate::heap::PrivateNameRef;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host, Termination};
use crate::value::{ObjectRef, Value};
use swell_compiler::bytecode::Instruction;

impl<H: Host> Agent<H> {
    /// The arm of every instruction of private names: reads the running instruction again (the
    /// one before the program counter, which the loop moved past it) and sends it to the
    /// function of its own below. Not inlined and given no argument, so the arm adds nothing to
    /// the frame of the loop (`Agent::execute`), which stays on the Rust stack at every
    /// re-entry. Another instruction never reaches it: a fault of the agent.
    #[inline(never)]
    pub(super) fn private_instruction(&mut self) -> Completion<Flow> {
        use Instruction as I;
        let instruction = {
            let frame = self.top_frame()?;
            frame
                .pc
                .checked_sub(1)
                .and_then(|at| frame.instructions().get(at).copied())
        };
        match instruction {
            Some(I::GetPrivate { hops, slot }) => self.get_private(hops, slot),
            Some(I::SetPrivate { hops, slot }) => self.set_private(hops, slot),
            Some(I::AddPrivateMethod {
                hops,
                slot,
                function_hops,
                function_slot,
            }) => self.add_private_method(
                u32::from(hops),
                u32::from(slot),
                u32::from(function_hops),
                u32::from(function_slot),
            ),
            Some(I::NewPrivateName { slot, name }) => self.new_private_name_instruction(slot, name),
            _ => Err(self.fault()),
        }
    }

    /// The Private Name in slot `slot` of the environment `hops` outward from the running
    /// frame's LexicalEnvironment.
    fn private_name_at(&mut self, hops: u32, slot: u32) -> Completion<PrivateNameRef> {
        let env = self.top_frame()?.env;
        match self.slot_state(env, hops, slot)? {
            SlotValue::PrivateName(name) => Ok(name),
            SlotValue::Value(_) | SlotValue::Uninitialized => Err(self.fault()),
        }
    }

    /// The object a private reference with base `base` acts on: ToObject (`#sec-toobject`)
    /// throws for `undefined` and `null`, and the wrapper of any other primitive has no private
    /// element, so PrivateElementFind would find nothing (`#sec-privateget`, step 2;
    /// `#sec-privateset`, step 2): a `TypeError` for every primitive, `what` the access.
    fn private_base(
        &mut self,
        base: Value,
        key: PrivateNameRef,
        what: &str,
    ) -> Completion<ObjectRef> {
        if let Value::Object(object) = base {
            return Ok(object);
        }
        let name = self.private_name_text(key);
        let message = match base {
            Value::Undefined => format!("cannot {what} the private member {name} of undefined"),
            Value::Null => format!("cannot {what} the private member {name} of null"),
            _ => format!("cannot {what} the private member {name} of a primitive value"),
        };
        Err(self.throw_error(ErrorKind::TypeError, &message))
    }

    /// `GetPrivate`: `[o] -> [v]`, GetValue of `o.#x` (`#sec-getvalue`, step 3.b: PrivateGet).
    #[inline(never)]
    fn get_private(&mut self, hops: u32, slot: u32) -> Completion<Flow> {
        let key = self.private_name_at(hops, slot)?;
        let base = self.peek_value(0)?;
        let object = self.private_base(base, key, "read")?;
        let value = self.private_get(object, key)?;
        self.replace(1, value)
    }

    /// `SetPrivate`: `[o, v] -> [v]`, PutValue of `o.#x` (`#sec-putvalue`, step 3.b:
    /// PrivateSet).
    #[inline(never)]
    fn set_private(&mut self, hops: u32, slot: u32) -> Completion<Flow> {
        let key = self.private_name_at(hops, slot)?;
        let base = self.peek_value(1)?;
        let value = self.peek_value(0)?;
        let object = self.private_base(base, key, "write")?;
        self.private_set(object, key, value)?;
        self.replace(2, value)
    }

    /// `AddPrivateMethod`: `[o] -> [o]`, PrivateMethodOrAccessorAdd of the function in slot
    /// `function_slot` of the environment `function_hops` outward, keyed by the Private Name in
    /// slot `slot` of the environment `hops` outward. `o` is `this` of the initializer of the
    /// instance elements or F, always an object.
    #[inline(never)]
    fn add_private_method(
        &mut self,
        hops: u32,
        slot: u32,
        function_hops: u32,
        function_slot: u32,
    ) -> Completion<Flow> {
        let key = self.private_name_at(hops, slot)?;
        let env = self.top_frame()?.env;
        let function = self.get_slot(env, function_hops, function_slot)?;
        let function = self.expect_object(function)?;
        let object = self.peek_value(0)?;
        let object = self.expect_object(object)?;
        self.private_method_or_accessor_add(object, key, function)?;
        Ok(Flow::Next)
    }

    /// `NewPrivateName`: a new Private Name whose [[Description]] is String constant `name`, into
    /// slot `slot` of the running frame's LexicalEnvironment, the private record just pushed
    /// (`#sec-runtime-semantics-classdefinitionevaluation`, step 7.b). Charged before it exists:
    /// over the budget the run ends in `Terminated(MemoryBudget)`.
    #[inline(never)]
    fn new_private_name_instruction(&mut self, slot: u32, name: u32) -> Completion<Flow> {
        let description = self.constant_string(name)?;
        let Ok(private) = self.heap.new_private_name(description) else {
            return Err(self.terminate(Termination::MemoryBudget));
        };
        let Some(env) = self.top_frame()?.env else {
            return Err(self.fault());
        };
        let index = usize::try_from(slot).unwrap_or(usize::MAX);
        let stored = self
            .heap
            .environments
            .get_mut(env)
            .ok()
            .and_then(|record| record.slots.get_mut(index))
            .map(|binding| *binding = SlotValue::PrivateName(private));
        match stored {
            Some(()) => Ok(Flow::Next),
            None => Err(self.fault()),
        }
    }
}
