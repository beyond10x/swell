//! The instructions that bind the parameters of a list that is not simple
//! (`docs/design/engine.md`, decision 3, "A parameter list that is not simple";
//! `swell_compiler::bytecode`, "parameters of a list that is not simple"), each in a function of
//! its own that the arm of `interp/dispatch.rs` calls, so the frame of the instruction loop
//! stays as it was. Written by `story:default-and-rest-parameters`.
//!
//! The call of such a function keeps its arguments in the frame, in the local slots after the
//! frame's own (`object/ecmascript_function.rs`), and its code binds each parameter in order
//! (FunctionDeclarationInstantiation, `#sec-functiondeclarationinstantiation`, step 26, with
//! IteratorBindingInitialization of the FormalParameters,
//! `#sec-runtime-semantics-iteratorbindinginitialization`): the argument, a default evaluated
//! only for `undefined`, or the rest Array, made by CreateArrayFromList
//! (`#sec-createarrayfromlist`) with a step per element and charged to the memory budget. A
//! function whose parameters have an initializer then makes the record of its body's `var`
//! names, which is its VariableEnvironment from there on (step 28).

use crate::env::{NameTable, SlotNames};
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::{Completion, Termination};
use crate::interp::frame::SlotValue;
use crate::value::{ObjectRef, Value};
use alloc::vec::Vec;

impl<H: Host> Agent<H> {
    /// `Argument(n)`: the argument the call kept at local slot `n`, `undefined` past the last.
    #[inline(never)]
    pub(crate) fn argument(&mut self, slot: u32) -> Completion<()> {
        let position = usize::try_from(slot).unwrap_or(usize::MAX);
        let value = match self.top_frame()?.locals.get(position) {
            Some(SlotValue::Value(value)) => *value,
            Some(SlotValue::Uninitialized) => return Err(self.fault()),
            None => Value::Undefined,
        };
        self.push_value(value)
    }

    /// `JumpIfNotUndefined(t)`: a value that is not `undefined` stays and the run continues at
    /// `t`; `undefined` is dropped.
    #[inline(never)]
    pub(crate) fn jump_if_not_undefined(&mut self, target: u32) -> Completion<()> {
        if matches!(self.peek_value(0)?, Value::Undefined) {
            return self.drop_entries(1);
        }
        self.top_frame()?.pc = usize::try_from(target).unwrap_or(usize::MAX);
        Ok(())
    }

    /// `RestArguments(n)`: an Array of the arguments the call kept from local slot `n` on. The
    /// list of their values is charged while it lives; CreateArrayFromList charges a step per
    /// element and the Array.
    #[inline(never)]
    pub(crate) fn rest_arguments(&mut self, slot: u32) -> Completion<()> {
        let position = usize::try_from(slot).unwrap_or(usize::MAX);
        let count = self.top_frame()?.locals.len().saturating_sub(position);
        let bytes = u64::try_from(count.saturating_mul(size_of::<Value>())).unwrap_or(u64::MAX);
        if self.heap.accounting.charge(bytes).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        let mut values = Vec::new();
        if values.try_reserve_exact(count).is_err() {
            self.heap.accounting.credit(bytes);
            return Err(self.terminate(Termination::MemoryBudget));
        }
        if let Some(frame) = self.interp.frames.last() {
            let kept = frame.locals.get(position..).unwrap_or_default();
            values.extend(kept.iter().filter_map(|binding| binding.value()));
        }
        let array: Completion<ObjectRef> = self.create_array_from_list(&values);
        drop(values);
        self.heap.accounting.credit(bytes);
        self.push_value(Value::Object(array?))
    }

    /// `PushVarEnv(t)`: a declarative record of one `undefined` slot per name of `scopes[t]` of
    /// the running code, which names its slots, becomes the LexicalEnvironment and the
    /// VariableEnvironment.
    #[inline(never)]
    pub(crate) fn push_var_env(&mut self, table: u32) -> Completion<()> {
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
        frame.var_env = Some(env);
        frame.pushed_envs = frame.pushed_envs.saturating_add(1);
        Ok(())
    }
}
