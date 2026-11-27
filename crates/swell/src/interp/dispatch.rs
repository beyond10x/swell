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

use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use swell_compiler::bytecode::{HandlerKind, Instruction, TAG_NORMAL, TAG_THROW};

use super::frame::{Entry, Enumerator, SlotValue};
use crate::env::{NameReference, NameTable, SlotNames};
use crate::heap::accounting::ChargedVec;
use crate::heap::arena::Handle;
use crate::host::agent::Agent;
use crate::host::outcome::{Abrupt, Completion};
use crate::host::{ErrorKind, Host};
use crate::object::ordinary::OrdinaryData;
use crate::object::property::{Descriptor, Key};
use crate::ops::objects::Enumeration;
use crate::ops::operators::{BinaryOperator, number_bitwise_not, number_unary_minus};
use crate::realm::intrinsics::Intrinsic;
use crate::value::{ObjectRef, StringRef, Value};

/// What an instruction asks of the loop; the other files of the interpreter that hold the work
/// of an instruction answer it too.
pub(crate) enum Flow {
    /// Go on with the running frame's next instruction (which may be a frame just pushed).
    Next,
    /// The running frame returns this value.
    Return(Value),
    /// The running frame was suspended into its holder (`interp/suspend.rs`), and this value is
    /// handed over as a return hands its value over.
    Suspend(Value),
}

impl<H: Host> Agent<H> {
    /// Run the frame stack until the frame at index `base` returns or is suspended
    /// (`interp/suspend.rs`), and answer its value; a throw no handler of those frames catches,
    /// or a termination, is the answer instead, with the frames above `base` dropped.
    pub(crate) fn run(&mut self, base: usize) -> Completion<Value> {
        loop {
            let flow = match self.safe_point() {
                Ok(()) => self.step().and_then(|()| self.execute_next()),
                Err(_) => Err(self.fault()),
            };
            let handed = match flow {
                Ok(Flow::Next) => continue,
                Ok(Flow::Return(value)) => self.return_from_frame(base, value),
                Ok(Flow::Suspend(value)) => self.hand_over(base, value),
                Err(abrupt) => Err(abrupt),
            };
            match handed {
                Ok(Some(value)) => return Ok(value),
                Ok(None) => {}
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
    /// enters no handler. A throw is first recorded where it arose (decision 13,
    /// `Agent::record_throw_site`), once per thrown value.
    fn unwind(&mut self, base: usize, abrupt: Abrupt) -> Option<Completion<Value>> {
        let Abrupt::Throw(value) = abrupt else {
            self.drop_frames(base);
            return Some(Err(abrupt));
        };
        self.record_throw_site(value);
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
    /// block) or the throw completion record (a `finally` block, and the close of an array
    /// pattern, whose stack is cut above its record, `interp/array_patterns.rs`). A `catch`
    /// block ends the throw's site record; the others keep it, since their end throws the
    /// value again.
    fn enter_handler(
        &mut self,
        handler: swell_compiler::bytecode::Handler,
        value: Value,
    ) -> Completion<()> {
        let mut items = usize::try_from(handler.items).unwrap_or(usize::MAX);
        if handler.kind == HandlerKind::Pattern {
            items = self.pattern_handler_items(items)?;
        }
        self.truncate_stack(items)?;
        let (mut env, mut pushed) = {
            let frame = self.top_frame()?;
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
        if handler.kind == HandlerKind::Catch {
            self.end_throw_site();
        } else {
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
    pub(super) fn constant_string(&mut self, index: u32) -> Completion<StringRef> {
        let position = usize::try_from(index).unwrap_or(usize::MAX);
        let found = self.top_frame()?.compiled.strings.get(position).copied();
        match found {
            Some(string) => Ok(string),
            None => Err(self.fault()),
        }
    }

    /// The property key of String constant `index`.
    pub(super) fn constant_key(&mut self, index: u32) -> Completion<Key> {
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
    pub(super) fn replace(&mut self, count: usize, value: Value) -> Completion<Flow> {
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

    /// One instruction. Each arm is a call: an instruction that needs locals of its own runs in
    /// a function of its own, so that the frame of this loop, which every re-entry puts on the
    /// thread's stack once more, holds what the dispatch needs and no instruction's locals
    /// (`Limits::min_stack_bytes`).
    fn execute(&mut self, instruction: Instruction, strict: bool) -> Completion<Flow> {
        use Instruction as I;
        match instruction {
            // ---- values ----
            I::Undefined => self.replace(0, Value::Undefined),
            I::Null => self.replace(0, Value::Null),
            I::True => self.replace(0, Value::Boolean(true)),
            I::False => self.replace(0, Value::Boolean(false)),
            I::Number(index) => self.number_constant(index),
            I::String(index) => self.string_constant(index),
            I::This => self.this_value(),
            I::NewTarget => self.new_target_value(),
            I::Closure(index) => self.closure(index),
            I::ArrowClosure(index) => self.arrow_closure(index),

            // ---- the operand stack ----
            I::Pop => self.pop_instruction(),
            I::Dup => self.dup(),
            I::Dup2 => self.dup2(),
            I::Bury(count) => self.bury(count),
            I::Nip(count) => self.nip(count),

            // ---- bindings ----
            I::GetLocal(slot) => self.get_local(slot),
            I::SetLocal(slot) => self.set_local(slot),
            I::GetEnv { hops, slot } => self.get_env(hops, slot),
            I::SetEnv { hops, slot } => self.set_env(hops, slot),
            I::AssignImmutable(index) => self.assign_immutable_instruction(index, strict),
            I::GetGlobal(index) => self.get_global(index, strict),
            I::ResolveGlobal(index) => self.resolve_global(index),
            I::GetGlobalRef(index) => self.get_global_ref(index, strict),
            I::PutGlobalRef(index) => self.put_global_ref(index, strict),
            I::TypeofGlobal(index) => self.typeof_global(index, strict),
            I::DeleteGlobal(index) => self.delete_global(index),

            // ---- names looked up at run time ----
            I::GetName(index) => self.get_name(index, strict),
            I::ResolveName(index) => self.resolve_name_entry(index),
            I::GetNameRef(index) => self.get_name_ref(index, strict),
            I::PutNameRef(index) => self.put_name_ref(index, strict),
            I::TypeofName(index) => self.typeof_name(index, strict),
            I::DeleteName(index) => self.delete_name_instruction(index),
            I::GetNameForCall(index) => self.get_name_for_call(index, strict),

            // ---- properties ----
            I::GetNamed(index) => self.get_named(index),
            I::PutNamed(index) => self.put_named(index, strict),
            I::GetIndex => self.get_index(),
            I::PutIndex => self.put_index(strict),
            I::PropertyKey => self.property_key(),
            I::DeleteNamed(index) => self.delete_named(index, strict),
            I::DeleteIndex => self.delete_index(strict),
            // ---- Super References (`interp/super_ref.rs`) ----
            I::SuperBase => self.super_base(),
            I::GetSuperNamed(index) => self.get_super_named(index),
            I::GetSuperIndex => self.get_super_index(),
            I::PutSuperNamed(index) => self.put_super_named(index, strict),
            I::PutSuperIndex => self.put_super_index(strict),
            I::SuperKey => self.super_key(),
            I::DeleteSuper => self.delete_super(),

            // ---- literals ----
            I::NewObject => self.new_object_literal(),
            I::NewArray => self.new_array_literal(),
            I::NewRegExp { body, flags } => self.new_regexp(body, flags),
            I::DefineField(index) => self.define_field(index),
            I::DefineGetter(index) => self.define_accessor(index, true),
            I::DefineSetter(index) => self.define_accessor(index, false),
            I::DefineMethod(index) => self.define_method(index),
            I::InitProto => self.init_proto(),
            I::DefineIndex(index) => self.define_index(index),
            I::ArrayLength(length) => self.set_array_length(length),
            // ---- computed keys and spreads (`interp/object_literal.rs`) ----
            I::ToPropertyKey => self.computed_key_instruction(),
            I::DefineComputedField => self.define_computed_field(),
            I::DefineComputedGetter => self.define_computed_function(true, false),
            I::DefineComputedSetter => self.define_computed_function(false, false),
            I::DefineComputedMethod => self.define_computed_function(false, true),
            I::SetFunctionName => self.set_function_name_instruction(),
            I::CopyDataProperties => self.copy_data_properties_instruction(),
            // ---- spread elements (`interp/spread.rs`) ----
            I::SpreadArray => self.spread_array(),
            I::ArrayAppend => self.array_append(),
            I::ArrayGrow(count) => self.array_grow(count),
            I::CallSpread => self.call_spread(),
            I::CallEvalSpread => self.call_eval_spread(strict),
            I::NewSpread => self.new_spread(),

            // ---- operators ----
            I::Typeof => self.typeof_value(),
            I::ToNumber => self.convert_to_number(),
            I::Negate => self.negate(),
            I::BitNot => self.bit_not(),
            I::Not => self.not(),
            I::ToNumeric => self.convert_to_numeric(),
            I::Increment => self.increment(true),
            I::Decrement => self.increment(false),
            I::Mul => self.binary(BinaryOperator::Multiply),
            I::Div => self.binary(BinaryOperator::Divide),
            I::Rem => self.binary(BinaryOperator::Remainder),
            I::Exp => self.binary(BinaryOperator::Exponentiate),
            I::Add => self.binary(BinaryOperator::Add),
            I::Sub => self.binary(BinaryOperator::Subtract),
            I::Shl => self.binary(BinaryOperator::LeftShift),
            I::Shr => self.binary(BinaryOperator::SignedRightShift),
            I::UShr => self.binary(BinaryOperator::UnsignedRightShift),
            I::BitAnd => self.binary(BinaryOperator::BitwiseAnd),
            I::BitXor => self.binary(BinaryOperator::BitwiseXor),
            I::BitOr => self.binary(BinaryOperator::BitwiseOr),
            I::Lt | I::Gt | I::LtEq | I::GtEq => self.relational(instruction),
            I::InstanceOf => self.instance_of(),
            I::In => self.in_instruction(),
            I::Eq => self.loose_equality(true),
            I::NotEq => self.loose_equality(false),
            I::StrictEq => self.strict_equality(true),
            I::StrictNotEq => self.strict_equality(false),

            // ---- calls ----
            I::Call(count) => self.call_instruction(count),
            I::CallEval(count) => self.call_eval_instruction(count, strict),
            I::New(count) => self.new_instruction(count),

            // ---- control ----
            I::Jump(target) => self.jump(target),
            I::JumpIfTrue(target) => self.jump_if(target, true),
            I::JumpIfFalse(target) => self.jump_if(target, false),
            I::LoopJump(target) => self.loop_jump(target),
            I::CaseJump(target) => self.case_jump(target),
            I::ForInStart => self.for_in_start(),
            I::ForInNext(target) => self.for_in_next(target),
            // `interp/for_of.rs`.
            I::GetIterator => self.get_iterator_instruction(),
            I::IteratorStep(target) => self.iterator_step_instruction(target),
            I::IteratorClose => self.iterator_close_instruction(),
            I::GetAsyncIterator => self.get_async_iterator_instruction(),
            I::AsyncIteratorNext => self.async_iterator_next_instruction(),
            I::AsyncIteratorStep(target) => self.async_iterator_step_instruction(target),
            I::AsyncIteratorClose(target) => self.async_iterator_close_instruction(target),
            I::AsyncIteratorClosed => self.async_iterator_closed_instruction(),
            I::Throw => self.throw_instruction(),
            I::ThrowSyntaxError(index) => self.throw_syntax_error(index),
            I::Return => self.return_instruction(),
            I::PushEnv(slots) => self.push_env(slots),
            I::PushNamedEnv(table) => self.push_named_env(table),
            I::PushWith => self.push_with(),
            I::PopEnv => self.pop_env(),
            I::Tag(tag) => self.tag_instruction(tag),
            I::JumpIfTag { tag, target } => self.jump_if_tag(tag, target),
            I::EndFinally => self.end_finally(),
            I::LoadCompletion => self.load_completion(),
            I::StoreCompletion => self.store_completion(),

            // ---- lexical bindings (`interp/lexical.rs`) ----
            I::Uninitialize(slot) => self.uninitialize(slot).map(|()| Flow::Next),
            I::PushLexicalEnv(table) => self.push_lexical_env(table).map(|()| Flow::Next),
            I::CopyEnv => self.copy_env().map(|()| Flow::Next),
            I::GetLocalChecked { slot, name } => {
                self.get_local_checked(slot, name).map(|()| Flow::Next)
            }
            I::SetLocalChecked { slot, name } => {
                self.set_local_checked(slot, name).map(|()| Flow::Next)
            }
            I::GetEnvChecked { hops, slot } => {
                self.get_env_checked(hops, slot).map(|()| Flow::Next)
            }
            I::SetEnvChecked { hops, slot } => {
                self.set_env_checked(hops, slot).map(|()| Flow::Next)
            }
            I::AssignConstLocal { slot, name } => {
                self.assign_const_local(slot, name).map(|()| Flow::Next)
            }
            I::AssignConstEnv { hops, slot } => {
                self.assign_const_env(hops, slot).map(|()| Flow::Next)
            }
            I::InitGlobal(index) => self.init_global(index).map(|()| Flow::Next),
            // ---- parameters of a list that is not simple (`interp/parameters.rs`) ----
            I::Argument(slot) => self.argument(slot).map(|()| Flow::Next),
            I::JumpIfNotUndefined(target) => {
                self.jump_if_not_undefined(target).map(|()| Flow::Next)
            }
            I::RestArguments(slot) => self.rest_arguments(slot).map(|()| Flow::Next),
            I::PushVarEnv(table) => self.push_var_env(table).map(|()| Flow::Next),
            // ---- object patterns (`interp/patterns.rs`) ----
            I::RequireObjectCoercible => {
                self.require_object_coercible_pattern().map(|()| Flow::Next)
            }
            I::PatternGet { depth, name } => self.pattern_get(depth, name).map(|()| Flow::Next),
            I::PatternGetKey { depth, kept } => self.pattern_get_key(depth, kept),
            I::ObjectRest { depth, kept } => self.object_rest(depth, kept),
            // ---- array patterns (`interp/array_patterns.rs`) ----
            I::PatternStep(depth) => self.pattern_step(depth),
            I::PatternSkip => self.pattern_skip(),
            I::PatternRest(depth) => self.pattern_rest(depth),
            // ---- templates (`interp/template.rs`) ----
            I::ToString => self.substitution_to_string(),
            // ---- tagged templates (`interp/templates.rs`) ----
            I::TemplateObject(site) => self.template_object(site),
            // ---- generators (`interp/generator.rs`) ----
            I::GeneratorStart(slot) => self.generator_start(slot),
            I::Yield(slot) => self.yield_instruction(slot),
            I::YieldReturn(items) => self.yield_return(items),
            I::GeneratorReturn(slot) => self.generator_return(slot),
            I::GeneratorDone(slot) => self.generator_done(slot),
            // ---- `yield*` (`interp/yield_star.rs`) ----
            I::DelegateStep(target) => self.delegate_step(target),
            I::JumpIfDelegateDone(target) => self.jump_if_delegate_done(target),
            I::DelegateYield(slot) => self.delegate_yield(slot),
            // ---- async functions (`interp/async_function.rs`) ----
            I::AsyncStart(slot) => self.async_start(slot),
            I::Await(slot) => self.await_instruction(slot),
            I::AsyncReturn(slot) => self.async_return(slot),
            I::AsyncReject(slot) => self.async_reject(slot),
            // ---- async generators (`interp/async_generator.rs`) ----
            I::AsyncGeneratorStart(slot) => self.async_generator_start(slot),
            I::AsyncGeneratorYield(slot) => self.async_generator_yield(slot),
            I::AsyncGeneratorReturn(slot) => self.async_generator_return(slot),
            I::AsyncGeneratorReject(slot) => self.async_generator_reject(slot),
            // ---- `yield*` in an async generator (`interp/yield_star.rs`) ----
            I::AsyncDelegateStep(target) => self.async_delegate_step(target),
            I::AsyncDelegateYield(slot) => self.async_delegate_yield(slot),
        }
    }

    // ---- values ----

    /// `Number(k)`.
    fn number_constant(&mut self, index: u32) -> Completion<Flow> {
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

    /// `String(k)`.
    fn string_constant(&mut self, index: u32) -> Completion<Flow> {
        let string = self.constant_string(index)?;
        self.replace(0, Value::String(string))
    }

    /// `This`.
    fn this_value(&mut self) -> Completion<Flow> {
        let this = self.top_frame()?.this;
        self.replace(0, this)
    }

    /// `NewTarget`: GetNewTarget (`#sec-getnewtarget`), the running frame's NewTarget or
    /// `undefined`.
    fn new_target_value(&mut self) -> Completion<Flow> {
        let new_target = self
            .top_frame()?
            .new_target
            .map_or(Value::Undefined, Value::Object);
        self.replace(0, new_target)
    }

    // ---- the operand stack ----

    /// `Pop`: the entry leaves through [`Agent::truncate_stack`], so an enumerator credits its
    /// key list.
    fn pop_instruction(&mut self) -> Completion<Flow> {
        self.drop_entries(1)?;
        Ok(Flow::Next)
    }

    /// `Dup`.
    fn dup(&mut self) -> Completion<Flow> {
        let value = self.peek_value(0)?;
        self.replace(0, value)
    }

    /// `Dup2`.
    fn dup2(&mut self) -> Completion<Flow> {
        let below = self.peek_value(1)?;
        let top = self.peek_value(0)?;
        self.push_value(below)?;
        self.replace(0, top)
    }

    /// `Bury(n)`.
    fn bury(&mut self, count: u32) -> Completion<Flow> {
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

    /// `Nip(n)`.
    fn nip(&mut self, count: u32) -> Completion<Flow> {
        let count = usize::try_from(count).unwrap_or(usize::MAX);
        let top = self.pop_entry()?;
        self.drop_entries(count)?;
        self.push_entry(top)?;
        Ok(Flow::Next)
    }

    // ---- bindings ----

    /// `GetLocal(s)`: an uninitialized slot is one the compiler did not check, a broken
    /// invariant.
    fn get_local(&mut self, slot: u32) -> Completion<Flow> {
        let position = usize::try_from(slot).unwrap_or(usize::MAX);
        let value = self.top_frame()?.locals.get(position).copied();
        match value {
            Some(SlotValue::Value(value)) => self.replace(0, value),
            Some(SlotValue::Uninitialized) | None => Err(self.fault()),
        }
    }

    /// `SetLocal(s)`.
    fn set_local(&mut self, slot: u32) -> Completion<Flow> {
        let value = self.peek_value(0)?;
        let position = usize::try_from(slot).unwrap_or(usize::MAX);
        let stored = self
            .top_frame()?
            .locals
            .get_mut(position)
            .map(|binding| *binding = SlotValue::Value(value));
        match stored {
            Some(()) => Ok(Flow::Next),
            None => Err(self.fault()),
        }
    }

    /// `GetEnv { hops, slot }`.
    fn get_env(&mut self, hops: u32, slot: u32) -> Completion<Flow> {
        let env = self.top_frame()?.env;
        let value = self.get_slot(env, hops, slot)?;
        self.replace(0, value)
    }

    /// `SetEnv { hops, slot }`.
    fn set_env(&mut self, hops: u32, slot: u32) -> Completion<Flow> {
        let value = self.peek_value(0)?;
        let env = self.top_frame()?.env;
        self.set_slot(env, hops, slot, value)?;
        Ok(Flow::Next)
    }

    /// `AssignImmutable(k)`.
    fn assign_immutable_instruction(&mut self, index: u32, strict: bool) -> Completion<Flow> {
        let name = self.constant_text(index);
        self.assign_immutable(strict, &name)?;
        Ok(Flow::Next)
    }

    /// `GetGlobal(k)`.
    fn get_global(&mut self, index: u32, strict: bool) -> Completion<Flow> {
        let key = self.constant_key(index)?;
        let realm = self.top_frame()?.realm;
        if !self.global_has_binding(realm, key)? {
            return Err(self.not_defined(index));
        }
        let value = self.global_get_binding_value(realm, key, strict)?;
        self.replace(0, value)
    }

    /// `ResolveGlobal(k)`.
    fn resolve_global(&mut self, index: u32) -> Completion<Flow> {
        let key = self.constant_key(index)?;
        let realm = self.top_frame()?.realm;
        let resolvable = self.global_has_binding(realm, key)?;
        self.push_entry(Entry::Reference(resolvable))?;
        Ok(Flow::Next)
    }

    /// `GetGlobalRef(k)`.
    fn get_global_ref(&mut self, index: u32, strict: bool) -> Completion<Flow> {
        if !self.top_reference(0)? {
            return Err(self.not_defined(index));
        }
        let key = self.constant_key(index)?;
        let realm = self.top_frame()?.realm;
        let value = self.global_get_binding_value(realm, key, strict)?;
        self.replace(0, value)
    }

    /// `PutGlobalRef(k)`.
    fn put_global_ref(&mut self, index: u32, strict: bool) -> Completion<Flow> {
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

    /// `TypeofGlobal(k)`.
    fn typeof_global(&mut self, index: u32, strict: bool) -> Completion<Flow> {
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

    /// `DeleteGlobal(k)`.
    fn delete_global(&mut self, index: u32) -> Completion<Flow> {
        let key = self.constant_key(index)?;
        let realm = self.top_frame()?.realm;
        let deleted = if self.global_has_binding(realm, key)? {
            self.global_delete_binding(realm, key)?
        } else {
            true
        };
        self.replace(0, Value::Boolean(deleted))
    }

    // ---- properties ----

    /// `GetNamed(k)`.
    fn get_named(&mut self, index: u32) -> Completion<Flow> {
        let base = self.peek_value(0)?;
        let key = self.constant_key(index)?;
        let value = self.get_value(base, key)?;
        self.replace(1, value)
    }

    /// `PutNamed(k)`.
    fn put_named(&mut self, index: u32, strict: bool) -> Completion<Flow> {
        let base = self.peek_value(1)?;
        let value = self.peek_value(0)?;
        let key = self.constant_key(index)?;
        self.put_value(base, key, value, strict)?;
        self.replace(2, value)
    }

    /// `GetIndex`.
    fn get_index(&mut self) -> Completion<Flow> {
        let base = self.peek_value(1)?;
        let key = self.peek_value(0)?;
        let value = self.get_value_computed(base, key)?;
        self.replace(2, value)
    }

    /// `PutIndex`.
    fn put_index(&mut self, strict: bool) -> Completion<Flow> {
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
            return Err(self.throw_error(ErrorKind::TypeError, "Cannot assign to the property"));
        }
        self.replace(3, value)
    }

    /// `PropertyKey`.
    fn property_key(&mut self) -> Completion<Flow> {
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

    /// `DeleteNamed(k)`.
    fn delete_named(&mut self, index: u32, strict: bool) -> Completion<Flow> {
        let base = self.peek_value(0)?;
        let key = self.constant_key(index)?;
        let deleted = self.delete_reference(base, key, strict)?;
        self.replace(1, Value::Boolean(deleted))
    }

    /// `DeleteIndex`.
    fn delete_index(&mut self, strict: bool) -> Completion<Flow> {
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

    /// `NewObject`.
    fn new_object_literal(&mut self) -> Completion<Flow> {
        let prototype = self.current_intrinsic(Intrinsic::ObjectPrototype)?;
        let object = self.ordinary_object_create(Some(prototype), OrdinaryData::Plain)?;
        self.replace(0, Value::Object(object))
    }

    /// `NewArray`.
    fn new_array_literal(&mut self) -> Completion<Flow> {
        let array = self.array_create(0.0, None)?;
        self.replace(0, Value::Object(array))
    }

    /// `DefineField(k)`.
    fn define_field(&mut self, index: u32) -> Completion<Flow> {
        let object = self.peek_value(1)?;
        let object = self.expect_object(object)?;
        let value = self.peek_value(0)?;
        let key = self.constant_key(index)?;
        self.create_data_property_or_throw(object, key, value)?;
        self.drop_entries(1)?;
        Ok(Flow::Next)
    }

    /// `DefineGetter(k)` (`getter` true) and `DefineSetter(k)`.
    fn define_accessor(&mut self, index: u32, getter: bool) -> Completion<Flow> {
        let object = self.peek_value(1)?;
        let object = self.expect_object(object)?;
        let function = self.peek_value(0)?;
        let function = self.expect_object(function)?;
        let key = self.constant_key(index)?;
        self.make_method(function, object)?;
        let descriptor = if getter {
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

    /// `DefineMethod(k)`: `#sec-runtime-semantics-definemethod` (MakeMethod; the closure is of
    /// kind method, so it has no [[Construct]]) and `#sec-definemethodproperty` with enumerable
    /// true.
    fn define_method(&mut self, index: u32) -> Completion<Flow> {
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

    /// `InitProto`.
    fn init_proto(&mut self) -> Completion<Flow> {
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

    /// `DefineIndex(i)`.
    fn define_index(&mut self, index: u32) -> Completion<Flow> {
        let array = self.peek_value(1)?;
        let array = self.expect_object(array)?;
        let value = self.peek_value(0)?;
        let key = self.index_key(f64::from(index))?;
        self.create_data_property_or_throw(array, key, value)?;
        self.drop_entries(1)?;
        Ok(Flow::Next)
    }

    /// `ArrayLength(n)`.
    fn set_array_length(&mut self, length: u32) -> Completion<Flow> {
        let array = self.peek_value(0)?;
        let array = self.expect_object(array)?;
        let key = self.name_key("length")?;
        self.set(array, key, Value::Number(f64::from(length)), true)?;
        Ok(Flow::Next)
    }

    // ---- operators ----

    /// `Typeof`.
    fn typeof_value(&mut self) -> Completion<Flow> {
        let value = self.peek_value(0)?;
        let kind = self.type_of(value)?;
        let string = self.atom(kind)?;
        self.replace(1, Value::String(string))
    }

    /// `ToNumber`.
    fn convert_to_number(&mut self) -> Completion<Flow> {
        let value = self.peek_value(0)?;
        let number = self.to_number(value)?;
        self.replace(1, Value::Number(number))
    }

    /// `Negate`.
    fn negate(&mut self) -> Completion<Flow> {
        let value = self.peek_value(0)?;
        let number = self.numeric_number(value)?;
        self.replace(1, Value::Number(number_unary_minus(number)))
    }

    /// `BitNot`.
    fn bit_not(&mut self) -> Completion<Flow> {
        let value = self.peek_value(0)?;
        let number = self.numeric_number(value)?;
        self.replace(1, Value::Number(number_bitwise_not(number)))
    }

    /// `Not`.
    fn not(&mut self) -> Completion<Flow> {
        let value = self.peek_value(0)?;
        let truth = self.to_boolean(value)?;
        self.replace(1, Value::Boolean(!truth))
    }

    /// `ToNumeric`.
    fn convert_to_numeric(&mut self) -> Completion<Flow> {
        let value = self.peek_value(0)?;
        let numeric = self.to_numeric(value)?;
        self.replace(1, numeric)
    }

    /// `Increment` (`up` true) and `Decrement`.
    fn increment(&mut self, up: bool) -> Completion<Flow> {
        let value = self.peek_value(0)?;
        let Value::Number(number) = value else {
            // ToNumeric ran before: no BigInt exists before slice 12.
            return Err(self.fault());
        };
        let changed = if up { number + 1.0 } else { number - 1.0 };
        self.replace(1, Value::Number(changed))
    }

    /// `InstanceOf`.
    fn instance_of(&mut self) -> Completion<Flow> {
        let value = self.peek_value(1)?;
        let target = self.peek_value(0)?;
        let result = self.instanceof_operator(value, target)?;
        self.replace(2, Value::Boolean(result))
    }

    /// `In`.
    fn in_instruction(&mut self) -> Completion<Flow> {
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

    /// `Eq` (`equal` true) and `NotEq`.
    fn loose_equality(&mut self, equal: bool) -> Completion<Flow> {
        let left = self.peek_value(1)?;
        let right = self.peek_value(0)?;
        let result = self.is_loosely_equal(left, right)? == equal;
        self.replace(2, Value::Boolean(result))
    }

    /// `StrictEq` (`equal` true) and `StrictNotEq`.
    fn strict_equality(&mut self, equal: bool) -> Completion<Flow> {
        let left = self.peek_value(1)?;
        let right = self.peek_value(0)?;
        let result = self.is_strictly_equal(left, right)? == equal;
        self.replace(2, Value::Boolean(result))
    }

    // ---- control ----

    /// `JumpIfTrue(t)` (`when` true) and `JumpIfFalse(t)`.
    fn jump_if(&mut self, target: u32, when: bool) -> Completion<Flow> {
        let value = self.peek_value(0)?;
        let truth = self.to_boolean(value)?;
        self.drop_entries(1)?;
        if truth == when {
            return self.jump(target);
        }
        Ok(Flow::Next)
    }

    /// `LoopJump(t)`.
    fn loop_jump(&mut self, target: u32) -> Completion<Flow> {
        self.check_interrupt()?;
        self.jump(target)
    }

    /// `CaseJump(t)`.
    fn case_jump(&mut self, target: u32) -> Completion<Flow> {
        let discriminant = self.peek_value(1)?;
        let value = self.peek_value(0)?;
        if self.is_strictly_equal(discriminant, value)? {
            self.drop_entries(2)?;
            return self.jump(target);
        }
        self.drop_entries(1)?;
        Ok(Flow::Next)
    }

    /// `Throw`.
    fn throw_instruction(&mut self) -> Completion<Flow> {
        let value = self.pop_value()?;
        Err(Abrupt::Throw(value))
    }

    /// `ThrowSyntaxError(k)`; a refusal of a later slice is
    /// counted ([`Agent::refusals`]).
    fn throw_syntax_error(&mut self, index: u32) -> Completion<Flow> {
        let message = self.constant_text(index);
        self.count_refusal(&message);
        Err(self.throw_error(ErrorKind::SyntaxError, &message))
    }

    /// `Return`.
    fn return_instruction(&mut self) -> Completion<Flow> {
        let value = self.pop_value()?;
        self.check_interrupt()?;
        Ok(Flow::Return(value))
    }

    /// `PushEnv(n)`.
    fn push_env(&mut self, slots: u32) -> Completion<Flow> {
        let outer = self.top_frame()?.env;
        let env = self.new_declarative_environment(outer, slots, None)?;
        let frame = self.top_frame()?;
        frame.env = Some(env);
        frame.pushed_envs = frame.pushed_envs.saturating_add(1);
        Ok(Flow::Next)
    }

    /// `PopEnv`.
    fn pop_env(&mut self) -> Completion<Flow> {
        let Some(env) = self.top_frame()?.env else {
            return Err(self.fault());
        };
        let outer = self.outer_environment(env)?;
        let frame = self.top_frame()?;
        frame.env = outer;
        frame.pushed_envs = frame.pushed_envs.saturating_sub(1);
        Ok(Flow::Next)
    }

    /// `Tag(t)`.
    fn tag_instruction(&mut self, tag: u32) -> Completion<Flow> {
        self.push_entry(Entry::Tag(tag))?;
        Ok(Flow::Next)
    }

    /// `JumpIfTag { tag, target }`.
    fn jump_if_tag(&mut self, tag: u32, target: u32) -> Completion<Flow> {
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

    /// `EndFinally`.
    fn end_finally(&mut self) -> Completion<Flow> {
        let tag = self.pop_entry()?;
        let value = self.pop_value()?;
        match tag {
            Entry::Tag(TAG_NORMAL) => Ok(Flow::Next),
            Entry::Tag(TAG_THROW) => Err(Abrupt::Throw(value)),
            _ => Err(self.fault()),
        }
    }

    /// `LoadCompletion`.
    fn load_completion(&mut self) -> Completion<Flow> {
        let value = self.top_frame()?.completion;
        self.replace(0, value)
    }

    /// `StoreCompletion`.
    fn store_completion(&mut self) -> Completion<Flow> {
        let value = self.pop_value()?;
        self.top_frame()?.completion = value;
        Ok(Flow::Next)
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
                compiled: alloc::rc::Rc::clone(&frame.compiled),
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
    pub(super) fn get_value(&mut self, base: Value, key: Key) -> Completion<Value> {
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
                alloc::rc::Rc::clone(&frame.compiled),
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
    /// returns or throws. The frame of a call on this loop is made in
    /// [`Agent::push_call_frame`], so that what it takes is not on the stack while a call from
    /// Rust runs. `this` and the function are read where they are; the n arguments are copied
    /// once (`docs/design/engine.md`, "Memory budget"): into a plain list the frame takes over
    /// at once, or, for a call from Rust, into a list charged before it is made
    /// ([`Agent::charged_arguments`]).
    fn call_instruction(&mut self, count: u32) -> Completion<Flow> {
        let count = usize::try_from(count).unwrap_or(usize::MAX);
        let function = self.peek_value(count)?;
        let Value::Object(object) = function else {
            return Err(self.throw_error(ErrorKind::TypeError, "the value is not a function"));
        };
        if !self.has_call(object)? {
            return Err(self.throw_error(ErrorKind::TypeError, "the object is not a function"));
        }
        if self.is_ecmascript_function(object)? {
            return self.call_frame_of_instruction(object, count);
        }
        // A call from Rust: the copy is dropped, and so credited, when this returns, on every
        // way out.
        let this = self.peek_value(count.saturating_add(1))?;
        let args = self.charged_arguments(count)?;
        let result = self.internal_call(object, this, &args);
        let result = self.check_interrupt_after(result)?;
        self.replace(count.saturating_add(2), result)
    }

    /// The call of the ECMAScript function `function` by `Call(count)`: `this` and the
    /// arguments read for [`Agent::push_call_frame`]. Not inlined, so that the frame of
    /// [`Agent::call_instruction`], alive while a call from Rust runs, holds none of it.
    #[inline(never)]
    fn call_frame_of_instruction(&mut self, function: ObjectRef, count: usize) -> Completion<Flow> {
        let this = self.peek_value(count.saturating_add(1))?;
        let args = self.peek_arguments(count)?;
        self.push_call_frame(function, this, &args, count)
    }

    /// The `count` arguments on top of the operand stack, bottom first, left there, in a plain
    /// list: for a frame that copies them into its own charged slots before any code runs.
    fn peek_arguments(&mut self, count: usize) -> Completion<Vec<Value>> {
        #[cfg(test)]
        note_instruction_copy(count);
        let frame = self.top_frame()?;
        let start = frame.stack.len().checked_sub(count);
        let values: Option<Vec<Value>> = start.and_then(|start| {
            frame.stack.get(start..).map(|entries| {
                entries
                    .iter()
                    .filter_map(|entry| match entry {
                        Entry::Value(value) => Some(*value),
                        _ => None,
                    })
                    .collect()
            })
        });
        match values {
            Some(values) if values.len() == count => Ok(values),
            _ => Err(self.fault()),
        }
    }

    /// The `count` arguments on top of the operand stack, bottom first, left there, in a list
    /// charged to the memory budget before it is made and credited when it drops. It charges
    /// no step: each value was pushed by an instruction that spent one.
    fn charged_arguments(&mut self, count: usize) -> Completion<ChargedVec<Value>> {
        #[cfg(test)]
        note_instruction_copy(count);
        let mut args = self.new_charged_vec(count)?;
        let frame = self.top_frame()?;
        let start = frame.stack.len().checked_sub(count);
        let copied = start
            .and_then(|start| frame.stack.get(start..))
            .is_some_and(|entries| {
                entries.iter().all(|entry| match entry {
                    Entry::Value(value) => args.push(*value).is_ok(),
                    _ => false,
                })
            });
        if !copied {
            return Err(self.fault());
        }
        Ok(args)
    }

    /// `CallEval(n)`: a call whose callee is the IdentifierReference `eval`
    /// (`#sec-function-calls-runtime-semantics-evaluation`, step 6). When the function is
    /// `%eval%` of the current realm (SameValue), the direct `eval`: `undefined` with no
    /// argument (step 6.a.ii), otherwise PerformEval of the first argument with the strictness
    /// of this code, direct (`dynamic_code.rs`), whose eval code gets a frame on this loop that
    /// replaces the `count + 2` entries of the call; any other function is called as `Call(n)`
    /// calls it. Not inlined, so that the run loop's frame holds none of its locals.
    #[inline(never)]
    fn call_eval_instruction(&mut self, count: u32, strict: bool) -> Completion<Flow> {
        let entries = usize::try_from(count)
            .unwrap_or(usize::MAX)
            .saturating_add(2);
        let function = self.peek_value(entries.saturating_sub(2))?;
        let realm = self.top_frame()?.realm;
        let eval = self.intrinsic(realm, Intrinsic::Eval)?;
        if function != Value::Object(eval) {
            return self.call_instruction(count);
        }
        if count == 0 {
            return self.replace(entries, Value::Undefined);
        }
        let source = self.peek_value(entries.saturating_sub(3))?;
        self.drop_entries(entries)?;
        // The source stays held by the activation the hooks run under, then by the eval code's
        // frame; a value the eval answers at once goes where the call's entries were.
        let keep = self.keep(source);
        let answer = self.direct_eval(eval, source, strict);
        self.release(keep);
        match answer? {
            Some(value) => {
                // No frame was pushed: the value of the call.
                self.push_value(value)?;
                Ok(Flow::Next)
            }
            None => Ok(Flow::Next),
        }
    }

    /// The call of the ECMAScript function `function` of `Call(count)` on this loop: its frame
    /// replaces the `count + 2` entries of the call.
    pub(super) fn push_call_frame(
        &mut self,
        function: ObjectRef,
        this: Value,
        args: &[Value],
        count: usize,
    ) -> Completion<Flow> {
        self.check_frame_depth()?;
        self.step()?;
        let frame = self.make_frame(function, this, args, None, None)?;
        self.drop_entries(count.saturating_add(2))?;
        self.push_frame(frame);
        self.check_interrupt()?;
        Ok(Flow::Next)
    }

    /// `New(n)`: EvaluateNew (`#sec-evaluatenew`). The frame of a construction on this loop is
    /// made in [`Agent::push_construct_frame`], as for `Call(n)`, and the arguments are copied
    /// once, as `Call(n)` copies them.
    fn new_instruction(&mut self, count: u32) -> Completion<Flow> {
        let count = usize::try_from(count).unwrap_or(usize::MAX);
        let constructor = self.peek_value(count)?;
        if !self.is_constructor(constructor)? {
            return Err(self.throw_error(ErrorKind::TypeError, "the value is not a constructor"));
        }
        let object = self.expect_object(constructor)?;
        if self.is_ecmascript_function(object)? {
            return self.construct_frame_of_instruction(object, count);
        }
        let args = self.charged_arguments(count)?;
        let result = self.construct_value(object, &args, None);
        let result = self.check_interrupt_after(result)?;
        self.replace(count.saturating_add(1), result)
    }

    /// The construction by the ECMAScript function `constructor` of `New(count)`: the
    /// arguments read for [`Agent::push_construct_frame`], as
    /// [`Agent::call_frame_of_instruction`] reads them.
    #[inline(never)]
    fn construct_frame_of_instruction(
        &mut self,
        constructor: ObjectRef,
        count: usize,
    ) -> Completion<Flow> {
        let args = self.peek_arguments(count)?;
        self.push_construct_frame(constructor, &args, count)
    }

    /// The construction by the ECMAScript function `constructor` of `New(count)` on this loop:
    /// its frame, with the new object as its `this` and `constructor` as its NewTarget, replaces
    /// the `count + 1` entries of the call.
    pub(super) fn push_construct_frame(
        &mut self,
        constructor: ObjectRef,
        args: &[Value],
        count: usize,
    ) -> Completion<Flow> {
        self.check_frame_depth()?;
        let this = self.ordinary_create_from_constructor(
            constructor,
            Intrinsic::ObjectPrototype,
            OrdinaryData::Plain,
        )?;
        let keep = self.keep(Value::Object(this));
        let new_target = Some(constructor);
        let frame = self.step().and_then(|()| {
            self.make_frame(constructor, Value::Undefined, args, Some(this), new_target)
        });
        self.release(keep);
        let frame = frame?;
        self.drop_entries(count.saturating_add(1))?;
        self.push_frame(frame);
        self.check_interrupt()?;
        Ok(Flow::Next)
    }

    /// `ForInStart`: ForIn/OfHeadEvaluation for `enumerate`
    /// (`#sec-runtime-semantics-forinofheadevaluation`). The charge of the key list
    /// EnumerateObjectProperties answers joins the frame's `charged` before the enumerator is
    /// pushed, so on every way out after the list exists the frame holds it, and
    /// [`Agent::truncate_stack`] credits it when the enumerator leaves the operand stack.
    fn for_in_start(&mut self) -> Completion<Flow> {
        let value = self.peek_value(0)?;
        let enumerator = match value {
            Value::Undefined | Value::Null => Enumerator {
                object: None,
                keys: Vec::new(),
                next: 0,
                bytes: 0,
                chain: ChargedVec::new(),
            },
            _ => {
                let object = self.to_object(value)?;
                let keep = self.keep(Value::Object(object));
                let enumeration = self.enumerate_object_properties(object);
                self.release(keep);
                let Enumeration { found: list, chain } = enumeration?;
                match self.interp.frames.last_mut() {
                    Some(frame) => frame.charged = frame.charged.saturating_add(list.bytes),
                    None => {
                        self.drop_key_list(list);
                        return Err(self.fault());
                    }
                }
                Enumerator {
                    object: Some(object),
                    bytes: list.bytes,
                    keys: list.keys,
                    next: 0,
                    chain,
                }
            }
        };
        self.drop_entries(1)?;
        self.push_entry(Entry::Enumerator(Box::new(enumerator)))?;
        Ok(Flow::Next)
    }

    /// `ForInNext(t)`: the next key of the enumerator that is still a property of the object
    /// ("A property that is deleted before it is processed is ignored",
    /// `#sec-enumerate-object-properties`), a step per key passed over, or the jump when none
    /// is left. Whether it is still one is read by [[GetOwnProperty]], never by a `has` trap:
    /// along the chain the walk recorded when it met a Proxy ([`Agent::recorded_chain_has`]:
    /// the loop follows the chain its walk recorded, once), else along [[GetPrototypeOf]] from
    /// the object (`Agent::enumeration_has_property`).
    fn for_in_next(&mut self, target: u32) -> Completion<Flow> {
        loop {
            let next = {
                let frame = self.top_frame()?;
                match frame.stack.last_mut() {
                    Some(Entry::Enumerator(enumerator)) => {
                        let key = enumerator.keys.get(enumerator.next).copied();
                        enumerator.next = enumerator.next.saturating_add(1);
                        Some((key, enumerator.object, !enumerator.chain.is_empty()))
                    }
                    _ => None,
                }
            };
            let (key, object, recorded) = match next {
                Some((Some(key), Some(object), recorded)) => (key, object, recorded),
                Some((None, _, _)) | Some((Some(_), None, _)) => return self.jump(target),
                None => return Err(self.fault()),
            };
            let present = if recorded {
                self.recorded_chain_has(key)?
            } else {
                self.enumeration_has_property(object, key)?
            };
            if present {
                let value = self.key_value(key)?;
                self.push_value(value)?;
                return Ok(Flow::Next);
            }
            self.charge_steps(1)?;
        }
    }

    /// Whether `key` is still a property when its turn comes, over the chain of objects the walk
    /// of the enumerator on top of the running frame's operand stack recorded (the loop
    /// follows the chain the walk found, once): [[GetOwnProperty]] of each object of it in order,
    /// and a step before each object after the first, as the walk along [[GetPrototypeOf]]
    /// charged a step per prototype; no [[GetPrototypeOf]] runs, so each object of the chain is
    /// asked for its prototype once, by the walk. The enumerator holds the chain and `key` while
    /// each object's method runs (`Frame::trace`), and is read again after each, so nothing is
    /// kept here.
    pub(crate) fn recorded_chain_has(&mut self, key: Key) -> Completion<bool> {
        let mut index = 0usize;
        loop {
            let link = {
                let frame = self.top_frame()?;
                match frame.stack.last() {
                    Some(Entry::Enumerator(enumerator)) => {
                        Some(enumerator.chain.get(index).copied())
                    }
                    _ => None,
                }
            };
            let object = match link {
                Some(Some(object)) => object,
                Some(None) => return Ok(false),
                None => return Err(self.fault()),
            };
            if index > 0 {
                self.charge_steps(1)?;
            }
            if self.get_own_property(object, key)?.is_some() {
                return Ok(true);
            }
            index = index.saturating_add(1);
        }
    }
}

/// The counting build: `Call(n)` or `New(n)` copied `count` values off the operand stack.
#[cfg(test)]
fn note_instruction_copy(count: usize) {
    crate::object::ecmascript_function::INSTRUCTION_ARGUMENTS_COPIED.with(|copied| {
        let count = u64::try_from(count).unwrap_or(u64::MAX);
        copied.set(copied.get().saturating_add(count));
    });
}

#[cfg(test)]
mod tests {
    use crate::host::hooks::test_host::agent_with_realm;
    use crate::{Outcome, Parsed, Source};
    use alloc::format;
    use alloc::string::ToString;
    use alloc::vec::Vec;

    /// The site records `source`, run as a script in a fresh agent, wrote, and whether it threw.
    fn records(source: &str) -> (u64, bool) {
        let (mut agent, realm) = agent_with_realm();
        let script = match agent.parse_script(&realm, Source::Utf8(source), None) {
            Ok(Parsed::Parsed(script)) => script,
            _ => panic!("{source:?} did not parse"),
        };
        let threw = match agent.evaluate_script(&script) {
            Ok(Outcome::Throw(_)) => true,
            Ok(Outcome::Normal(_)) => false,
            _ => panic!("{source:?} was terminated"),
        };
        (agent.site_records, threw)
    }

    /// One record per throw that unwinds (decision 13): a throw that unwinds through three
    /// script frames, a `map` callback and a `finally` block writes one; a throw a `catch` ended
    /// and a second throw write two, of the same value too, and so do a throw a `finally` block
    /// discarded and a second `throw` statement of the same value; a run that throws nothing
    /// writes none.
    #[test]
    fn one_site_record_per_throw() {
        let through = "function a() { [1].map(function () { b(); }); }\n\
                       function b() { try { c(); } finally {} }\n\
                       function c() { null.x; }\n\
                       a();";
        assert_eq!(records(through), (1, true));
        assert_eq!(records("try { throw 1 } catch (e) {} throw 2"), (2, true));
        assert_eq!(records("try { throw 1 } catch (e) {} throw 1"), (2, true));
        let discarded = "function f() { try { throw 1 } finally { return } }\nf();\nthrow 1";
        assert_eq!(records(discarded), (2, true));
        // 10,000 iterations; 100 in the mode `collect-every-safe-point`, which collects at every
        // instruction (10,000 took 27 s there).
        let iterations = if cfg!(feature = "collect-every-safe-point") {
            100
        } else {
            10_000
        };
        let loop_ = format!("for (var i = 0; i < {iterations}; i++) {{ i * 2; }}");
        assert_eq!(records(&loop_), (0, false));
    }

    std::thread_local! {
        /// `used()` at the last two calls of `probe`, the last one second.
        static PROBED: core::cell::Cell<(u64, u64)> = const { core::cell::Cell::new((0, 0)) };
        /// The values `Call(n)` and `New(n)` copied before the last call of `probe`.
        static COPIED_AT_PROBE: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
        /// `used()` and the values copied when `h` was last entered, and its depth.
        static IN_H: core::cell::Cell<(u64, u64, u32)> = const { core::cell::Cell::new((0, 0, 0)) };
    }

    /// The values the two instructions copied since the last reset.
    fn copied() -> u64 {
        crate::object::ecmascript_function::INSTRUCTION_ARGUMENTS_COPIED.with(core::cell::Cell::get)
    }

    /// The host function `probe`: records `used()` and the values copied, then counts the
    /// copies again from 0.
    fn probe(
        cx: &mut crate::host::Cx<'_, crate::host::hooks::test_host::NoHooks>,
        _call: crate::host::HostCall<'_>,
    ) -> crate::host::Completion<crate::Value> {
        let used = cx.agent.heap.accounting.used();
        PROBED.with(|probed| probed.set((probed.get().1, used)));
        COPIED_AT_PROBE.with(|at| at.set(copied()));
        crate::object::ecmascript_function::INSTRUCTION_ARGUMENTS_COPIED.with(|c| c.set(0));
        Ok(crate::Value::Undefined)
    }

    /// The host function `h`: records `used()` and the values copied, then calls its first
    /// argument, a function, until it is entered the fourth time. Under `new` it answers
    /// itself, an object.
    fn h(
        cx: &mut crate::host::Cx<'_, crate::host::hooks::test_host::NoHooks>,
        call: crate::host::HostCall<'_>,
    ) -> crate::host::Completion<crate::Value> {
        let used = cx.agent.heap.accounting.used();
        let depth = IN_H.with(|in_h| in_h.get().2) + 1;
        IN_H.with(|in_h| in_h.set((used, copied(), depth)));
        if depth < 4
            && let Some(&function) = call.args.first()
            && matches!(function, crate::Value::Object(_))
        {
            cx.call(function, crate::Value::Undefined, &[]).unwrap()?;
        }
        Ok(match call.new_target {
            Some(_) => crate::Value::Object(call.callee),
            None => crate::Value::Undefined,
        })
    }

    /// `docs/design/engine.md`, "Memory budget": `Call(n)` and `New(n)` copy the n arguments of
    /// a builtin or host function once, into a list charged before it is made and dropped
    /// when the call returns (EvaluateCall, `#sec-evaluatecall`; EvaluateNew,
    /// `#sec-evaluatenew`), and an ECMAScript callee's once, with nothing new charged. A host
    /// function entered with 50 arguments reads the instruction's copy and the activation's
    /// charged; each instruction copies 50 values; four levels through a call back into script
    /// leave no capacity uncovered at an entry; an ECMAScript callee reads what it read before;
    /// the steps are the base's.
    #[test]
    fn call_and_new_hold_one_charged_copy_of_their_arguments_across_a_call() {
        use crate::builtins::string::transform::tests::Answer;
        use crate::object::string::across::Across;
        use crate::object::string::copies;
        let value = u64::try_from(size_of::<crate::Value>()).unwrap();
        let numbers = (2..=49)
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let mut across = Across::with_steps(1_000_000);
        across.function("probe", probe, false);
        across.function("h", h, true);
        let mut steps = Vec::new();
        for (source, counted) in [
            (format!("h(1, {numbers}, probe())"), 50),
            (format!("new h(1, {numbers}, probe())"), 50),
        ] {
            IN_H.with(|in_h| in_h.set((0, 0, 0)));
            assert!(matches!(across.run(&source), Answer::Normal(_)), "{source}");
            let (inside, copied, _) = IN_H.with(core::cell::Cell::get);
            let probed = PROBED.with(|probed| probed.get().1);
            assert!(
                inside - probed >= 2 * 50 * value,
                "{source}: {}",
                inside - probed
            );
            assert_eq!(copied, counted, "{source}");
            steps.push(across.steps());
        }
        IN_H.with(|in_h| in_h.set((0, 0, 0)));
        let source =
            format!("function g() {{ h(g, {numbers}, probe()); }} h(g, {numbers}, probe())");
        assert!(matches!(across.run(&source), Answer::Normal(_)));
        assert_eq!(IN_H.with(|in_h| in_h.get().2), 4);
        assert_eq!(copies::uncovered_at_entry(), 0);
        steps.push(across.steps());
        let parameters = (1..=50)
            .map(|n| format!("a{n}"))
            .collect::<Vec<_>>()
            .join(", ");
        let source =
            format!("function e({parameters}) {{ probe(); }} probe(); e(1, {numbers}, 50)");
        assert!(matches!(across.run(&source), Answer::Normal(_)));
        let (first, second) = PROBED.with(core::cell::Cell::get);
        assert_eq!(COPIED_AT_PROBE.with(core::cell::Cell::get), 50);
        assert_eq!(second - first, CALLEE_RISE);
        steps.push(across.steps());
        assert_eq!(steps, CALL_ACROSS_STEPS);
    }

    /// The rise of `used()` from before the call of the ECMAScript callee to its first
    /// statement on the base, before the story: 2,032 bytes; 2,048 since
    /// `story:new-target-meta-property` added the frame's NewTarget (`size_of::<Frame>()` from
    /// 208 to 224 bytes).
    const CALLEE_RISE: u64 = 2_048;

    /// The steps of the four scripts of
    /// [`call_and_new_hold_one_charged_copy_of_their_arguments_across_a_call`] on the base.
    const CALL_ACROSS_STEPS: [u64; 4] = [61, 60, 247, 73];
}
