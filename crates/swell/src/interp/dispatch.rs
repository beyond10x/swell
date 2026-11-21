//! The instruction loop of the interpreter (`docs/design/engine.md`, decision 3). Filled by
//! `story:interpreter-and-script-entry-points`.
//!
//! [`Agent::run`] runs the frame stack above a base until the base frame returns: one arm per
//! instruction of `swell-compiler` (`swell_compiler::bytecode`, whose documentation states each
//! instruction's stack effect and clause), a step per instruction, a safe point at every
//! instruction boundary (decision 4), the handler tables for a throw, and for a termination no
//! handler at all: the frames above the base are dropped and the termination is the answer.
//!
//! Every operand an operation that may run code (a getter, a setter, `valueOf`, `toString`, a
//! call) still needs stays on the operand stack, which the collector scans, until the operation
//! is done; a value made inside an instruction that a later step of it needs across such an
//! operation is kept on the agent's kept stack (`object/builtin_function.rs`).

use swell_compiler::bytecode::{HandlerKind, Instruction, TAG_NORMAL, TAG_THROW};

use super::frame::{Entry, Enumerator};
use crate::env::{NameReference, NameTable, SlotNames};
use crate::heap::arena::Handle;
use crate::host::agent::Agent;
use crate::host::outcome::{Abrupt, Completion};
use crate::host::{ErrorKind, Host};
use crate::object::ordinary::OrdinaryData;
use crate::object::property::{Descriptor, Key};
use crate::ops::operators::{BinaryOperator, number_bitwise_not, number_unary_minus};
use crate::realm::intrinsics::Intrinsic;
use crate::value::{ObjectRef, StringRef, Value};

/// What an instruction asks of the loop.
enum Flow {
    /// Go on with the running frame's next instruction (which may be a frame just pushed).
    Next,
    /// The running frame returns this value.
    Return(Value),
}

impl<H: Host> Agent<H> {
    /// Run the frame stack until the frame at index `base` returns, and answer its value; a
    /// throw no handler of those frames catches, or a termination, is the answer instead, with
    /// the frames above `base` dropped.
    pub(crate) fn run(&mut self, base: usize) -> Completion<Value> {
        loop {
            let flow = match self.safe_point() {
                Ok(()) => self.step().and_then(|()| self.execute_next()),
                Err(_) => Err(self.fault()),
            };
            match flow {
                Ok(Flow::Next) => {}
                Ok(Flow::Return(value)) => match self.return_from_frame(base, value) {
                    Ok(Some(value)) => return Ok(value),
                    Ok(None) => {}
                    Err(abrupt) => {
                        if let Some(answer) = self.unwind(base, abrupt) {
                            return answer;
                        }
                    }
                },
                Err(abrupt) => {
                    if let Some(answer) = self.unwind(base, abrupt) {
                        return answer;
                    }
                }
            }
        }
    }

    /// The return of the running frame with `value` ([[Call]] step 8 and [[Construct]] step 10:
    /// a constructor's code that returns no object answers its `this`). `Some` when it was the
    /// base frame; otherwise the value goes on the caller's operand stack, and the interrupt is
    /// checked.
    fn return_from_frame(&mut self, base: usize, value: Value) -> Completion<Option<Value>> {
        let frame = self.pop_frame()?;
        let value = match (frame.constructed, value) {
            (Some(this), value) if !matches!(value, Value::Object(_)) => Value::Object(this),
            _ => value,
        };
        if self.interp.frames.len() <= base {
            return Ok(Some(value));
        }
        let realm = self.top_frame()?.realm;
        self.set_realm_slot(Some(realm));
        self.push_value(value)?;
        self.check_interrupt()?;
        Ok(None)
    }

    /// A throw or a termination in the running frame: the first handler of the frames above
    /// `base` that covers the instruction, or the answer of the run (`Some`). A termination
    /// enters no handler.
    fn unwind(&mut self, base: usize, abrupt: Abrupt) -> Option<Completion<Value>> {
        let Abrupt::Throw(value) = abrupt else {
            self.drop_frames(base);
            return Some(Err(abrupt));
        };
        loop {
            let Some(frame) = self.interp.frames.last() else {
                return Some(Err(self.fault()));
            };
            let at = u32::try_from(frame.pc.saturating_sub(1)).unwrap_or(u32::MAX);
            let handler = frame
                .handlers()
                .iter()
                .find(|handler| handler.start <= at && at < handler.end)
                .copied();
            if let Some(handler) = handler {
                return match self.enter_handler(handler, value) {
                    Ok(()) => None,
                    Err(abrupt) => {
                        self.drop_frames(base);
                        Some(Err(abrupt))
                    }
                };
            }
            if self.interp.frames.len() <= base.saturating_add(1) {
                self.drop_frames(base);
                return Some(Err(Abrupt::Throw(value)));
            }
            self.drop_frames(self.interp.frames.len().saturating_sub(1));
            if let Some(frame) = self.interp.frames.last() {
                let realm = frame.realm;
                self.set_realm_slot(Some(realm));
            }
        }
    }

    /// Continue at `handler` of the running frame with the thrown `value`: the operand stack and
    /// the environments as they were at the try statement, then the exception (a `catch`
    /// block) or the throw completion record (a `finally` block).
    fn enter_handler(
        &mut self,
        handler: swell_compiler::bytecode::Handler,
        value: Value,
    ) -> Completion<()> {
        let items = usize::try_from(handler.items).unwrap_or(usize::MAX);
        let (mut env, mut pushed) = {
            let frame = self.top_frame()?;
            if frame.stack.len() < items {
                return Err(self.fault());
            }
            frame.stack.truncate(items);
            (frame.env, frame.pushed_envs)
        };
        while pushed > handler.envs {
            let Some(handle) = env else {
                return Err(self.fault());
            };
            env = self.outer_environment(handle)?;
            pushed -= 1;
        }
        let frame = self.top_frame()?;
        frame.env = env;
        frame.pushed_envs = pushed;
        frame.pc = usize::try_from(handler.target).unwrap_or(usize::MAX);
        let realm = frame.realm;
        self.set_realm_slot(Some(realm));
        self.push_value(value)?;
        if handler.kind == HandlerKind::Finally {
            self.push_entry(Entry::Tag(TAG_THROW))?;
        }
        Ok(())
    }

    /// Fetch the running frame's next instruction and run it.
    fn execute_next(&mut self) -> Completion<Flow> {
        let (instruction, strict) = {
            let frame = self.top_frame()?;
            let pc = frame.pc;
            let instruction = frame.instructions().get(pc).copied();
            frame.pc = pc.saturating_add(1);
            (instruction, frame.strict)
        };
        match instruction {
            Some(instruction) => self.execute(instruction, strict),
            None => Err(self.fault()),
        }
    }

    /// The String constant `index` of the running frame's code.
    fn constant_string(&mut self, index: u32) -> Completion<StringRef> {
        let position = usize::try_from(index).unwrap_or(usize::MAX);
        let found = self.top_frame()?.compiled.strings.get(position).copied();
        match found {
            Some(string) => Ok(string),
            None => Err(self.fault()),
        }
    }

    /// The property key of String constant `index`.
    fn constant_key(&mut self, index: u32) -> Completion<Key> {
        let position = usize::try_from(index).unwrap_or(usize::MAX);
        let found = self.top_frame()?.compiled.keys.get(position).copied();
        match found {
            Some(key) => Ok(key),
            None => Err(self.fault()),
        }
    }

    /// String constant `index` as Rust text, for a message.
    fn constant_text(&mut self, index: u32) -> String {
        let position = usize::try_from(index).unwrap_or(usize::MAX);
        self.interp
            .frames
            .last()
            .and_then(|frame| frame.compiled.code.strings.get(position))
            .map(|units| String::from_utf16_lossy(units))
            .unwrap_or_default()
    }

    /// Jump to instruction `target` of the running frame.
    fn jump(&mut self, target: u32) -> Completion<Flow> {
        self.top_frame()?.pc = usize::try_from(target).unwrap_or(usize::MAX);
        Ok(Flow::Next)
    }

    /// Replace the top `count` entries with `value`.
    fn replace(&mut self, count: usize, value: Value) -> Completion<Flow> {
        self.drop_entries(count)?;
        self.push_value(value)?;
        Ok(Flow::Next)
    }

    /// The atom of `text`.
    fn atom(&mut self, text: &str) -> Completion<StringRef> {
        let units: Vec<u16> = text.encode_utf16().collect();
        match self.heap.intern(&units) {
            Ok(atom) => Ok(atom),
            Err(_) => Err(self.terminate(crate::host::Termination::MemoryBudget)),
        }
    }

    /// One instruction.
    fn execute(&mut self, instruction: Instruction, strict: bool) -> Completion<Flow> {
        use Instruction as I;
        match instruction {
            // ---- values ----
            I::Undefined => self.replace(0, Value::Undefined),
            I::Null => self.replace(0, Value::Null),
            I::True => self.replace(0, Value::Boolean(true)),
            I::False => self.replace(0, Value::Boolean(false)),
            I::Number(index) => {
                let position = usize::try_from(index).unwrap_or(usize::MAX);
                let number = self
                    .top_frame()?
                    .compiled
                    .code
                    .numbers
                    .get(position)
                    .copied();
                match number {
                    Some(number) => self.replace(0, Value::Number(number)),
                    None => Err(self.fault()),
                }
            }
            I::String(index) => {
                let string = self.constant_string(index)?;
                self.replace(0, Value::String(string))
            }
            I::This => {
                let this = self.top_frame()?.this;
                self.replace(0, this)
            }
            I::Closure(index) => self.closure(index),

            // ---- the operand stack ----
            I::Pop => {
                self.pop_entry()?;
                Ok(Flow::Next)
            }
            I::Dup => {
                let value = self.peek_value(0)?;
                self.replace(0, value)
            }
            I::Dup2 => {
                let below = self.peek_value(1)?;
                let top = self.peek_value(0)?;
                self.push_value(below)?;
                self.replace(0, top)
            }
            I::Bury(count) => {
                let count = usize::try_from(count).unwrap_or(usize::MAX);
                let frame = self.top_frame()?;
                let len = frame.stack.len();
                match (frame.stack.pop(), len.checked_sub(count.saturating_add(1))) {
                    (Some(top), Some(at)) => {
                        frame.stack.insert(at, top);
                        Ok(Flow::Next)
                    }
                    _ => Err(self.fault()),
                }
            }
            I::Nip(count) => {
                let count = usize::try_from(count).unwrap_or(usize::MAX);
                let top = self.pop_entry()?;
                self.drop_entries(count)?;
                self.push_entry(top)?;
                Ok(Flow::Next)
            }

            // ---- bindings ----
            I::GetLocal(slot) => {
                let position = usize::try_from(slot).unwrap_or(usize::MAX);
                let value = self.top_frame()?.locals.get(position).copied();
                match value {
                    Some(value) => self.replace(0, value),
                    None => Err(self.fault()),
                }
            }
            I::SetLocal(slot) => {
                let value = self.peek_value(0)?;
                let position = usize::try_from(slot).unwrap_or(usize::MAX);
                let stored = self
                    .top_frame()?
                    .locals
                    .get_mut(position)
                    .map(|binding| *binding = value);
                match stored {
                    Some(()) => Ok(Flow::Next),
                    None => Err(self.fault()),
                }
            }
            I::GetEnv { hops, slot } => {
                let env = self.top_frame()?.env;
                let value = self.get_slot(env, hops, slot)?;
                self.replace(0, value)
            }
            I::SetEnv { hops, slot } => {
                let value = self.peek_value(0)?;
                let env = self.top_frame()?.env;
                self.set_slot(env, hops, slot, value)?;
                Ok(Flow::Next)
            }
            I::AssignImmutable(index) => {
                let name = self.constant_text(index);
                self.assign_immutable(strict, &name)?;
                Ok(Flow::Next)
            }
            I::GetGlobal(index) => {
                let key = self.constant_key(index)?;
                let realm = self.top_frame()?.realm;
                if !self.global_has_binding(realm, key)? {
                    return Err(self.not_defined(index));
                }
                let value = self.global_get_binding_value(realm, key, strict)?;
                self.replace(0, value)
            }
            I::ResolveGlobal(index) => {
                let key = self.constant_key(index)?;
                let realm = self.top_frame()?.realm;
                let resolvable = self.global_has_binding(realm, key)?;
                self.push_entry(Entry::Reference(resolvable))?;
                Ok(Flow::Next)
            }
            I::GetGlobalRef(index) => {
                if !self.top_reference(0)? {
                    return Err(self.not_defined(index));
                }
                let key = self.constant_key(index)?;
                let realm = self.top_frame()?.realm;
                let value = self.global_get_binding_value(realm, key, strict)?;
                self.replace(0, value)
            }
            I::PutGlobalRef(index) => {
                let value = self.peek_value(0)?;
                let resolvable = self.top_reference(1)?;
                let key = self.constant_key(index)?;
                let realm = self.top_frame()?.realm;
                if resolvable {
                    self.global_set_mutable_binding(realm, key, value, strict)?;
                } else if strict {
                    return Err(self.not_defined(index));
                } else {
                    let global = self.global_binding_object(realm)?;
                    self.set(global, key, value, false)?;
                }
                self.replace(2, value)
            }
            I::TypeofGlobal(index) => {
                let key = self.constant_key(index)?;
                let realm = self.top_frame()?.realm;
                let kind = if self.global_has_binding(realm, key)? {
                    let value = self.global_get_binding_value(realm, key, strict)?;
                    self.type_of(value)?
                } else {
                    "undefined"
                };
                let string = self.atom(kind)?;
                self.replace(0, Value::String(string))
            }
            I::DeleteGlobal(index) => {
                let key = self.constant_key(index)?;
                let realm = self.top_frame()?.realm;
                let deleted = if self.global_has_binding(realm, key)? {
                    self.global_delete_binding(realm, key)?
                } else {
                    true
                };
                self.replace(0, Value::Boolean(deleted))
            }

            // ---- names looked up at run time: each in a function of its own ----
            I::GetName(index) => self.get_name(index, strict),
            I::ResolveName(index) => self.resolve_name_entry(index),
            I::GetNameRef(index) => self.get_name_ref(index, strict),
            I::PutNameRef(index) => self.put_name_ref(index, strict),
            I::TypeofName(index) => self.typeof_name(index, strict),
            I::DeleteName(index) => self.delete_name_instruction(index),
            I::GetNameForCall(index) => self.get_name_for_call(index, strict),

            // ---- properties ----
            I::GetNamed(index) => {
                let base = self.peek_value(0)?;
                let key = self.constant_key(index)?;
                let value = self.get_value(base, key)?;
                self.replace(1, value)
            }
            I::PutNamed(index) => {
                let base = self.peek_value(1)?;
                let value = self.peek_value(0)?;
                let key = self.constant_key(index)?;
                self.put_value(base, key, value, strict)?;
                self.replace(2, value)
            }
            I::GetIndex => {
                let base = self.peek_value(1)?;
                let key = self.peek_value(0)?;
                let value = self.get_value_computed(base, key)?;
                self.replace(2, value)
            }
            I::PutIndex => {
                let base = self.peek_value(2)?;
                let key = self.peek_value(1)?;
                let value = self.peek_value(0)?;
                let object = self.require_object_coercible_and_wrap(base)?;
                let keep = self.keep(Value::Object(object));
                let key = self.to_property_key(key);
                let done = match key {
                    Ok(key) => {
                        let held = self.keep_key(key);
                        let done = self.internal_set(object, key, value, base);
                        self.release(held);
                        done
                    }
                    Err(abrupt) => Err(abrupt),
                };
                self.release(keep);
                if !done? && strict {
                    return Err(
                        self.throw_error(ErrorKind::TypeError, "Cannot assign to the property")
                    );
                }
                self.replace(3, value)
            }
            I::PropertyKey => {
                let base = self.peek_value(1)?;
                let key = self.peek_value(0)?;
                if matches!(base, Value::Undefined | Value::Null) {
                    return Err(self.throw_error(
                        ErrorKind::TypeError,
                        "Cannot read properties of undefined or null",
                    ));
                }
                let key = self.to_property_key(key)?;
                let key = self.key_value(key)?;
                self.replace(1, key)
            }
            I::DeleteNamed(index) => {
                let base = self.peek_value(0)?;
                let key = self.constant_key(index)?;
                let deleted = self.delete_reference(base, key, strict)?;
                self.replace(1, Value::Boolean(deleted))
            }
            I::DeleteIndex => {
                let base = self.peek_value(1)?;
                let key = self.peek_value(0)?;
                let object = self.require_object_coercible_and_wrap(base)?;
                let keep = self.keep(Value::Object(object));
                let deleted = match self.to_property_key(key) {
                    Ok(key) => self.delete_from(object, key, strict),
                    Err(abrupt) => Err(abrupt),
                };
                self.release(keep);
                self.replace(2, Value::Boolean(deleted?))
            }

            // ---- literals ----
            I::NewObject => {
                let prototype = self.current_intrinsic(Intrinsic::ObjectPrototype)?;
                let object = self.ordinary_object_create(Some(prototype), OrdinaryData::Plain)?;
                self.replace(0, Value::Object(object))
            }
            I::NewArray => {
                let array = self.array_create(0.0, None)?;
                self.replace(0, Value::Object(array))
            }
            I::DefineField(index) => {
                let object = self.peek_value(1)?;
                let object = self.expect_object(object)?;
                let value = self.peek_value(0)?;
                let key = self.constant_key(index)?;
                self.create_data_property_or_throw(object, key, value)?;
                self.drop_entries(1)?;
                Ok(Flow::Next)
            }
            I::DefineGetter(index) | I::DefineSetter(index) => {
                let object = self.peek_value(1)?;
                let object = self.expect_object(object)?;
                let function = self.peek_value(0)?;
                let function = self.expect_object(function)?;
                let key = self.constant_key(index)?;
                self.make_method(function, object)?;
                let descriptor = if matches!(instruction, I::DefineGetter(_)) {
                    Descriptor {
                        get: Some(Some(function)),
                        enumerable: Some(true),
                        configurable: Some(true),
                        ..Descriptor::default()
                    }
                } else {
                    Descriptor {
                        set: Some(Some(function)),
                        enumerable: Some(true),
                        configurable: Some(true),
                        ..Descriptor::default()
                    }
                };
                self.define_property_or_throw(object, key, descriptor)?;
                self.drop_entries(1)?;
                Ok(Flow::Next)
            }
            // `#sec-runtime-semantics-definemethod` (MakeMethod; the closure is of kind method, so
            // it has no [[Construct]]) and `#sec-definemethodproperty` with enumerable true.
            I::DefineMethod(index) => {
                let object = self.peek_value(1)?;
                let object = self.expect_object(object)?;
                let function = self.peek_value(0)?;
                let function = self.expect_object(function)?;
                let key = self.constant_key(index)?;
                self.make_method(function, object)?;
                let descriptor = Descriptor {
                    value: Some(Value::Object(function)),
                    writable: Some(true),
                    enumerable: Some(true),
                    configurable: Some(true),
                    ..Descriptor::default()
                };
                self.define_property_or_throw(object, key, descriptor)?;
                self.drop_entries(1)?;
                Ok(Flow::Next)
            }
            I::InitProto => {
                let object = self.peek_value(1)?;
                let object = self.expect_object(object)?;
                let value = self.peek_value(0)?;
                // `object` is the ordinary object the literal made, whose [[SetPrototypeOf]] is
                // OrdinarySetPrototypeOf; its result is not used ("Perform !").
                match value {
                    Value::Object(prototype) => {
                        self.ordinary_set_prototype_of(object, Some(prototype))?;
                    }
                    Value::Null => {
                        self.ordinary_set_prototype_of(object, None)?;
                    }
                    _ => {}
                }
                self.drop_entries(1)?;
                Ok(Flow::Next)
            }
            I::DefineIndex(index) => {
                let array = self.peek_value(1)?;
                let array = self.expect_object(array)?;
                let value = self.peek_value(0)?;
                let key = self.index_key(f64::from(index))?;
                self.create_data_property_or_throw(array, key, value)?;
                self.drop_entries(1)?;
                Ok(Flow::Next)
            }
            I::ArrayLength(length) => {
                let array = self.peek_value(0)?;
                let array = self.expect_object(array)?;
                let key = self.name_key("length")?;
                self.set(array, key, Value::Number(f64::from(length)), true)?;
                Ok(Flow::Next)
            }

            // ---- operators ----
            I::Typeof => {
                let value = self.peek_value(0)?;
                let kind = self.type_of(value)?;
                let string = self.atom(kind)?;
                self.replace(1, Value::String(string))
            }
            I::ToNumber => {
                let value = self.peek_value(0)?;
                let number = self.to_number(value)?;
                self.replace(1, Value::Number(number))
            }
            I::Negate => {
                let value = self.peek_value(0)?;
                let number = self.numeric_number(value)?;
                self.replace(1, Value::Number(number_unary_minus(number)))
            }
            I::BitNot => {
                let value = self.peek_value(0)?;
                let number = self.numeric_number(value)?;
                self.replace(1, Value::Number(number_bitwise_not(number)))
            }
            I::Not => {
                let value = self.peek_value(0)?;
                let truth = self.to_boolean(value)?;
                self.replace(1, Value::Boolean(!truth))
            }
            I::ToNumeric => {
                let value = self.peek_value(0)?;
                let numeric = self.to_numeric(value)?;
                self.replace(1, numeric)
            }
            I::Increment | I::Decrement => {
                let value = self.peek_value(0)?;
                let Value::Number(number) = value else {
                    // ToNumeric ran before: no BigInt exists before slice 12.
                    return Err(self.fault());
                };
                let changed = if matches!(instruction, I::Increment) {
                    number + 1.0
                } else {
                    number - 1.0
                };
                self.replace(1, Value::Number(changed))
            }
            I::Mul => self.binary(BinaryOperator::Multiply),
            I::Div => self.binary(BinaryOperator::Divide),
            I::Rem => self.binary(BinaryOperator::Remainder),
            I::Add => self.binary(BinaryOperator::Add),
            I::Sub => self.binary(BinaryOperator::Subtract),
            I::Shl => self.binary(BinaryOperator::LeftShift),
            I::Shr => self.binary(BinaryOperator::SignedRightShift),
            I::UShr => self.binary(BinaryOperator::UnsignedRightShift),
            I::BitAnd => self.binary(BinaryOperator::BitwiseAnd),
            I::BitXor => self.binary(BinaryOperator::BitwiseXor),
            I::BitOr => self.binary(BinaryOperator::BitwiseOr),
            I::Lt | I::Gt | I::LtEq | I::GtEq => self.relational(instruction),
            I::InstanceOf => {
                let value = self.peek_value(1)?;
                let target = self.peek_value(0)?;
                let result = self.instanceof_operator(value, target)?;
                self.replace(2, Value::Boolean(result))
            }
            I::In => {
                let key = self.peek_value(1)?;
                let target = self.peek_value(0)?;
                let Value::Object(object) = target else {
                    return Err(self.throw_error(
                        ErrorKind::TypeError,
                        "the right-hand side of 'in' is not an object",
                    ));
                };
                let key = self.to_property_key(key)?;
                let held = self.keep_key(key);
                let found = self.has_property(object, key);
                self.release(held);
                self.replace(2, Value::Boolean(found?))
            }
            I::Eq | I::NotEq => {
                let left = self.peek_value(1)?;
                let right = self.peek_value(0)?;
                let equal = self.is_loosely_equal(left, right)?;
                let result = if matches!(instruction, I::Eq) {
                    equal
                } else {
                    !equal
                };
                self.replace(2, Value::Boolean(result))
            }
            I::StrictEq | I::StrictNotEq => {
                let left = self.peek_value(1)?;
                let right = self.peek_value(0)?;
                let equal = self.is_strictly_equal(left, right)?;
                let result = if matches!(instruction, I::StrictEq) {
                    equal
                } else {
                    !equal
                };
                self.replace(2, Value::Boolean(result))
            }

            // ---- calls ----
            I::Call(count) => self.call_instruction(count),
            I::New(count) => self.new_instruction(count),

            // ---- control ----
            I::Jump(target) => self.jump(target),
            I::JumpIfTrue(target) | I::JumpIfFalse(target) => {
                let value = self.peek_value(0)?;
                let truth = self.to_boolean(value)?;
                self.drop_entries(1)?;
                if truth == matches!(instruction, I::JumpIfTrue(_)) {
                    return self.jump(target);
                }
                Ok(Flow::Next)
            }
            I::LoopJump(target) => {
                self.check_interrupt()?;
                self.jump(target)
            }
            I::CaseJump(target) => {
                let discriminant = self.peek_value(1)?;
                let value = self.peek_value(0)?;
                if self.is_strictly_equal(discriminant, value)? {
                    self.drop_entries(2)?;
                    return self.jump(target);
                }
                self.drop_entries(1)?;
                Ok(Flow::Next)
            }
            I::ForInStart => self.for_in_start(),
            I::ForInNext(target) => self.for_in_next(target),
            I::Throw => {
                let value = self.pop_value()?;
                Err(Abrupt::Throw(value))
            }
            I::ThrowSyntaxError(index) => {
                let message = self.constant_text(index);
                Err(self.throw_error(ErrorKind::SyntaxError, &message))
            }
            I::Return => {
                let value = self.pop_value()?;
                self.check_interrupt()?;
                Ok(Flow::Return(value))
            }
            I::PushEnv(slots) => {
                let outer = self.top_frame()?.env;
                let env = self.new_declarative_environment(outer, slots, None)?;
                let frame = self.top_frame()?;
                frame.env = Some(env);
                frame.pushed_envs = frame.pushed_envs.saturating_add(1);
                Ok(Flow::Next)
            }
            I::PushNamedEnv(table) => self.push_named_env(table),
            I::PushWith => self.push_with(),
            I::PopEnv => {
                let Some(env) = self.top_frame()?.env else {
                    return Err(self.fault());
                };
                let outer = self.outer_environment(env)?;
                let frame = self.top_frame()?;
                frame.env = outer;
                frame.pushed_envs = frame.pushed_envs.saturating_sub(1);
                Ok(Flow::Next)
            }
            I::Tag(tag) => {
                self.push_entry(Entry::Tag(tag))?;
                Ok(Flow::Next)
            }
            I::JumpIfTag { tag, target } => {
                let found = match self.top_frame()?.stack.last() {
                    Some(Entry::Tag(found)) => Some(*found),
                    _ => None,
                };
                match found {
                    Some(found) if found == tag => {
                        self.drop_entries(1)?;
                        self.jump(target)
                    }
                    Some(_) => Ok(Flow::Next),
                    None => Err(self.fault()),
                }
            }
            I::EndFinally => {
                let tag = self.pop_entry()?;
                let value = self.pop_value()?;
                match tag {
                    Entry::Tag(TAG_NORMAL) => Ok(Flow::Next),
                    Entry::Tag(TAG_THROW) => Err(Abrupt::Throw(value)),
                    _ => Err(self.fault()),
                }
            }
            I::LoadCompletion => {
                let value = self.top_frame()?.completion;
                self.replace(0, value)
            }
            I::StoreCompletion => {
                let value = self.pop_value()?;
                self.top_frame()?.completion = value;
                Ok(Flow::Next)
            }
        }
    }

    /// The `ReferenceError` of an unresolvable name, String constant `index`.
    fn not_defined(&mut self, index: u32) -> Abrupt {
        let name = self.constant_text(index);
        self.throw_error(ErrorKind::ReferenceError, &format!("{name} is not defined"))
    }

    /// Whether the global reference `depth` entries below the top is resolvable.
    fn top_reference(&mut self, depth: usize) -> Completion<bool> {
        let frame = self.top_frame()?;
        let found = frame
            .stack
            .len()
            .checked_sub(depth.saturating_add(1))
            .and_then(|index| frame.stack.get(index));
        match found {
            Some(Entry::Reference(resolvable)) => Ok(*resolvable),
            _ => Err(self.fault()),
        }
    }

    /// ResolveBinding of String constant `index` from the running frame's LexicalEnvironment
    /// (`env/mod.rs`): the reference, the name's key and the frame's realm.
    #[inline(never)]
    fn look_up(&mut self, index: u32) -> Completion<(NameReference, Key, Handle)> {
        let key = self.constant_key(index)?;
        let (env, realm) = {
            let frame = self.top_frame()?;
            (frame.env, frame.realm)
        };
        let reference = self.resolve_name(env, realm, key)?;
        Ok((reference, key, realm))
    }

    /// The Reference Record of a name `depth` entries below the top.
    fn name_reference_at(&mut self, depth: usize) -> Completion<NameReference> {
        let frame = self.top_frame()?;
        let found = frame
            .stack
            .len()
            .checked_sub(depth.saturating_add(1))
            .and_then(|index| frame.stack.get(index))
            .and_then(Entry::name_reference);
        match found {
            Some(reference) => Ok(reference),
            None => Err(self.fault()),
        }
    }

    /// GetValue of `reference` to String constant `index`: a `ReferenceError` when it is
    /// unresolvable.
    fn name_value_or_throw(
        &mut self,
        reference: NameReference,
        index: u32,
        key: Key,
        realm: Handle,
        strict: bool,
    ) -> Completion<Value> {
        match self.name_value(reference, realm, key, strict)? {
            Some(value) => Ok(value),
            None => Err(self.not_defined(index)),
        }
    }

    /// `GetName(k)`.
    #[inline(never)]
    fn get_name(&mut self, index: u32, strict: bool) -> Completion<Flow> {
        let (reference, key, realm) = self.look_up(index)?;
        let value = self.name_value_or_throw(reference, index, key, realm, strict)?;
        self.replace(0, value)
    }

    /// `ResolveName(k)`.
    #[inline(never)]
    fn resolve_name_entry(&mut self, index: u32) -> Completion<Flow> {
        let (reference, _, _) = self.look_up(index)?;
        self.push_entry(Entry::of_name(reference))?;
        Ok(Flow::Next)
    }

    /// `GetNameRef(k)`.
    #[inline(never)]
    fn get_name_ref(&mut self, index: u32, strict: bool) -> Completion<Flow> {
        let reference = self.name_reference_at(0)?;
        let key = self.constant_key(index)?;
        let realm = self.top_frame()?.realm;
        let value = self.name_value_or_throw(reference, index, key, realm, strict)?;
        self.replace(0, value)
    }

    /// `PutNameRef(k)`.
    #[inline(never)]
    fn put_name_ref(&mut self, index: u32, strict: bool) -> Completion<Flow> {
        let value = self.peek_value(0)?;
        let reference = self.name_reference_at(1)?;
        if reference == NameReference::Unresolvable && strict {
            return Err(self.not_defined(index));
        }
        let key = self.constant_key(index)?;
        let realm = self.top_frame()?.realm;
        self.put_name_value(reference, realm, key, value, strict)?;
        self.replace(2, value)
    }

    /// `TypeofName(k)`.
    #[inline(never)]
    fn typeof_name(&mut self, index: u32, strict: bool) -> Completion<Flow> {
        let (reference, key, realm) = self.look_up(index)?;
        let kind = match self.name_value(reference, realm, key, strict)? {
            Some(value) => self.type_of(value)?,
            None => "undefined",
        };
        let string = self.atom(kind)?;
        self.replace(0, Value::String(string))
    }

    /// `DeleteName(k)`.
    #[inline(never)]
    fn delete_name_instruction(&mut self, index: u32) -> Completion<Flow> {
        let (reference, key, realm) = self.look_up(index)?;
        let deleted = self.delete_name(reference, realm, key)?;
        self.replace(0, Value::Boolean(deleted))
    }

    /// `GetNameForCall(k)`: the function, and as the this value WithBaseObject of the record
    /// that binds the name (`#sec-evaluatecall`, step 1.b).
    #[inline(never)]
    fn get_name_for_call(&mut self, index: u32, strict: bool) -> Completion<Flow> {
        let (reference, key, realm) = self.look_up(index)?;
        let function = self.name_value_or_throw(reference, index, key, realm, strict)?;
        let this = match reference {
            NameReference::Object(object) => Value::Object(object),
            _ => Value::Undefined,
        };
        self.push_value(this)?;
        self.replace(0, function)
    }

    /// `PushNamedEnv(t)`: a declarative record of one slot per name of `scopes[t]` of the
    /// running code, which names its slots.
    #[inline(never)]
    fn push_named_env(&mut self, table: u32) -> Completion<Flow> {
        let (outer, names, slots) = {
            let frame = self.top_frame()?;
            let code = &frame.compiled.code;
            let scopes = match frame.template {
                Some(t) => code
                    .functions
                    .get(usize::try_from(t).unwrap_or(usize::MAX))
                    .map(|template| &template.scopes),
                None => Some(&code.scopes),
            };
            let slots = scopes
                .and_then(|scopes| scopes.get(usize::try_from(table).unwrap_or(usize::MAX)))
                .map(|names| u32::try_from(names.len()).unwrap_or(u32::MAX));
            let names = SlotNames {
                script: frame.script,
                compiled: std::rc::Rc::clone(&frame.compiled),
                table: NameTable::Scope {
                    template: frame.template,
                    index: table,
                },
            };
            (frame.env, names, slots)
        };
        let Some(slots) = slots else {
            return Err(self.fault());
        };
        let env = self.new_declarative_environment(outer, slots, Some(names))?;
        let frame = self.top_frame()?;
        frame.env = Some(env);
        frame.pushed_envs = frame.pushed_envs.saturating_add(1);
        Ok(Flow::Next)
    }

    /// `PushWith`: the start of a `with` statement
    /// (`#sec-with-statement-runtime-semantics-evaluation`, steps 2 to 5).
    #[inline(never)]
    fn push_with(&mut self) -> Completion<Flow> {
        let value = self.peek_value(0)?;
        if matches!(value, Value::Undefined | Value::Null) {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "the object of a with statement is undefined or null",
            ));
        }
        let object = self.to_object(value)?;
        let keep = self.keep(Value::Object(object));
        let outer = self.top_frame()?.env;
        let env = self.new_with_environment(object, outer);
        self.release(keep);
        let env = env?;
        self.drop_entries(1)?;
        let frame = self.top_frame()?;
        frame.env = Some(env);
        frame.pushed_envs = frame.pushed_envs.saturating_add(1);
        Ok(Flow::Next)
    }

    /// The object of a property reference's base (GetValue and PutValue, `#sec-getvalue`,
    /// `#sec-putvalue`: ToObject of [[Base]]), with a message for `undefined` and `null`.
    fn require_object_coercible_and_wrap(&mut self, base: Value) -> Completion<ObjectRef> {
        if matches!(base, Value::Undefined | Value::Null) {
            let which = if base == Value::Null {
                "null"
            } else {
                "undefined"
            };
            return Err(self.throw_error(
                ErrorKind::TypeError,
                &format!("Cannot use a property of {which}"),
            ));
        }
        self.to_object(base)
    }

    /// GetValue (`#sec-getvalue`) of the property reference `base[key]`: [[Get]] of ToObject of
    /// the base with the base itself as the receiver (GetThisValue).
    fn get_value(&mut self, base: Value, key: Key) -> Completion<Value> {
        let object = self.require_object_coercible_and_wrap(base)?;
        let keep = self.keep(Value::Object(object));
        let value = self.internal_get(object, key, base);
        self.release(keep);
        value
    }

    /// GetValue of `base[key]` with a key not yet converted: ToObject of the base, then
    /// ToPropertyKey of the key.
    fn get_value_computed(&mut self, base: Value, key: Value) -> Completion<Value> {
        let object = self.require_object_coercible_and_wrap(base)?;
        let keep = self.keep(Value::Object(object));
        let value = match self.to_property_key(key) {
            Ok(key) => {
                let held = self.keep_key(key);
                let value = self.internal_get(object, key, base);
                self.release(held);
                value
            }
            Err(abrupt) => Err(abrupt),
        };
        self.release(keep);
        value
    }

    /// PutValue (`#sec-putvalue`) of the property reference `base[key]`: [[Set]] on ToObject of
    /// the base with the base as the receiver; a refusal is a `TypeError` in strict code.
    fn put_value(&mut self, base: Value, key: Key, value: Value, strict: bool) -> Completion<()> {
        let object = self.require_object_coercible_and_wrap(base)?;
        let keep = self.keep(Value::Object(object));
        let done = self.internal_set(object, key, value, base);
        self.release(keep);
        if !done? && strict {
            return Err(self.throw_error(ErrorKind::TypeError, "Cannot assign to the property"));
        }
        Ok(())
    }

    /// The `delete` of a property reference
    /// (`#sec-delete-operator-runtime-semantics-evaluation`, step 5).
    fn delete_reference(&mut self, base: Value, key: Key, strict: bool) -> Completion<bool> {
        let object = self.require_object_coercible_and_wrap(base)?;
        let keep = self.keep(Value::Object(object));
        let deleted = self.delete_from(object, key, strict);
        self.release(keep);
        deleted
    }

    /// [[Delete]] of `key` of `object`; `false` is a `TypeError` in strict code.
    fn delete_from(&mut self, object: ObjectRef, key: Key, strict: bool) -> Completion<bool> {
        let held = self.keep_key(key);
        let deleted = self.internal_delete(object, key);
        self.release(held);
        let deleted = deleted?;
        if !deleted && strict {
            return Err(self.throw_error(ErrorKind::TypeError, "Cannot delete the property"));
        }
        Ok(deleted)
    }

    /// CreateDataPropertyOrThrow (`#sec-createdatapropertyorthrow`).
    fn create_data_property_or_throw(
        &mut self,
        object: ObjectRef,
        key: Key,
        value: Value,
    ) -> Completion<()> {
        if !self.create_data_property(object, key, value)? {
            return Err(self.throw_error(ErrorKind::TypeError, "Cannot define the property"));
        }
        Ok(())
    }

    /// The Number of ToNumeric of `value`: no BigInt exists before slice 12.
    fn numeric_number(&mut self, value: Value) -> Completion<f64> {
        match self.to_numeric(value)? {
            Value::Number(number) => Ok(number),
            _ => Err(self.fault()),
        }
    }

    /// `[a, b] -> [r]`: ApplyStringOrNumericBinaryOperator.
    fn binary(&mut self, operator: BinaryOperator) -> Completion<Flow> {
        let left = self.peek_value(1)?;
        let right = self.peek_value(0)?;
        let result = self.apply_string_or_numeric_binary_operator(left, operator, right)?;
        self.replace(2, result)
    }

    /// The relational operators (`#sec-relational-operators-runtime-semantics-evaluation`) with
    /// IsLessThan (`#sec-islessthan`): an undefined result is false.
    fn relational(&mut self, instruction: Instruction) -> Completion<Flow> {
        let left = self.peek_value(1)?;
        let right = self.peek_value(0)?;
        let result = match instruction {
            Instruction::Lt => self.is_less_than(left, right, true)? == Some(true),
            Instruction::Gt => self.is_less_than(right, left, false)? == Some(true),
            Instruction::LtEq => self.is_less_than(right, left, false)? == Some(false),
            _ => self.is_less_than(left, right, true)? == Some(false),
        };
        self.replace(2, Value::Boolean(result))
    }

    /// `Closure`: the function object of template `index` in the LexicalEnvironment.
    fn closure(&mut self, index: u32) -> Completion<Flow> {
        let (compiled, script, realm, env) = {
            let frame = self.top_frame()?;
            (
                std::rc::Rc::clone(&frame.compiled),
                frame.script,
                frame.realm,
                frame.env,
            )
        };
        let function = self.instantiate_function(&compiled, script, realm, index, env)?;
        self.replace(0, Value::Object(function))
    }

    /// `Call(n)`: EvaluateCall (`#sec-evaluatecall`). An ECMAScript function gets a frame on this
    /// loop; any other function is called from Rust, and the interrupt is checked when it
    /// returns.
    fn call_instruction(&mut self, count: u32) -> Completion<Flow> {
        let count = usize::try_from(count).unwrap_or(usize::MAX);
        let values = self.peek_values(count.saturating_add(2))?;
        let (this, function, args) = match values.as_slice() {
            [this, function, args @ ..] => (*this, *function, args.to_vec()),
            _ => return Err(self.fault()),
        };
        let Value::Object(object) = function else {
            return Err(self.throw_error(ErrorKind::TypeError, "the value is not a function"));
        };
        if !self.has_call(object)? {
            return Err(self.throw_error(ErrorKind::TypeError, "the object is not a function"));
        }
        if self.is_ecmascript_function(object)? {
            self.check_frame_depth()?;
            self.step()?;
            let frame = self.make_frame(object, this, &args, None)?;
            self.drop_entries(count.saturating_add(2))?;
            self.push_frame(frame);
            self.check_interrupt()?;
            return Ok(Flow::Next);
        }
        let result = self.internal_call(object, this, &args)?;
        self.check_interrupt()?;
        self.replace(count.saturating_add(2), result)
    }

    /// `New(n)`: EvaluateNew (`#sec-evaluatenew`).
    fn new_instruction(&mut self, count: u32) -> Completion<Flow> {
        let count = usize::try_from(count).unwrap_or(usize::MAX);
        let values = self.peek_values(count.saturating_add(1))?;
        let (constructor, args) = match values.as_slice() {
            [constructor, args @ ..] => (*constructor, args.to_vec()),
            _ => return Err(self.fault()),
        };
        if !self.is_constructor(constructor)? {
            return Err(self.throw_error(ErrorKind::TypeError, "the value is not a constructor"));
        }
        let object = self.expect_object(constructor)?;
        if self.is_ecmascript_function(object)? {
            self.check_frame_depth()?;
            let this = self.ordinary_create_from_constructor(
                object,
                Intrinsic::ObjectPrototype,
                OrdinaryData::Plain,
            )?;
            let keep = self.keep(Value::Object(this));
            let frame = self
                .step()
                .and_then(|()| self.make_frame(object, Value::Undefined, &args, Some(this)));
            self.release(keep);
            let frame = frame?;
            self.drop_entries(count.saturating_add(1))?;
            self.push_frame(frame);
            self.check_interrupt()?;
            return Ok(Flow::Next);
        }
        let result = self.construct_value(object, &args, None)?;
        self.check_interrupt()?;
        self.replace(count.saturating_add(1), result)
    }

    /// `ForInStart`: ForIn/OfHeadEvaluation for `enumerate`
    /// (`#sec-runtime-semantics-forinofheadevaluation`).
    fn for_in_start(&mut self) -> Completion<Flow> {
        let value = self.peek_value(0)?;
        let enumerator = match value {
            Value::Undefined | Value::Null => Enumerator {
                object: None,
                keys: Vec::new(),
                next: 0,
            },
            _ => {
                let object = self.to_object(value)?;
                let keep = self.keep(Value::Object(object));
                let keys = self.enumerate_object_properties(object);
                self.release(keep);
                Enumerator {
                    object: Some(object),
                    keys: keys?,
                    next: 0,
                }
            }
        };
        self.drop_entries(1)?;
        self.push_entry(Entry::Enumerator(Box::new(enumerator)))?;
        Ok(Flow::Next)
    }

    /// `ForInNext(t)`: the next key of the enumerator that is still a property of the object
    /// ("A property that is deleted before it is processed is ignored",
    /// `#sec-enumerate-object-properties`), a step per key passed over, or the jump when none is
    /// left.
    fn for_in_next(&mut self, target: u32) -> Completion<Flow> {
        loop {
            let next = {
                let frame = self.top_frame()?;
                match frame.stack.last_mut() {
                    Some(Entry::Enumerator(enumerator)) => {
                        let key = enumerator.keys.get(enumerator.next).copied();
                        enumerator.next = enumerator.next.saturating_add(1);
                        Some((key, enumerator.object))
                    }
                    _ => None,
                }
            };
            let (key, object) = match next {
                Some((Some(key), Some(object))) => (key, object),
                Some((None, _)) | Some((Some(_), None)) => return self.jump(target),
                None => return Err(self.fault()),
            };
            if self.has_property(object, key)? {
                let value = self.key_value(key)?;
                self.push_value(value)?;
                return Ok(Flow::Next);
            }
            self.charge_steps(1)?;
        }
    }
}
