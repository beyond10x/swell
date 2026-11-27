//! The instructions of `let` and `const` bindings (`docs/design/engine.md`, decision 3, "Lexical
//! bindings"; `swell_compiler::bytecode`, "lexical bindings"), each in a function of its own
//! that the arm of `interp/dispatch.rs` calls, so the frame of the instruction loop stays as it
//! was. Written by `story:let-const-and-block-scopes`.
//!
//! A binding is uninitialized from the entry of its scope until its declaration runs (the
//! temporal dead zone, `#sec-let-and-const-declarations`): a local slot or a record's slot holds
//! [`SlotValue::Uninitialized`], and an access the compiler could not prove to come after the
//! declaration is a checked one, a `ReferenceError` naming the binding while it is
//! uninitialized. An assignment to a `const` binding is a `TypeError` in any code. A scope whose
//! bindings a closure or a lookup by name reaches has a record of its own (`PushLexicalEnv`),
//! copied for each iteration of a `for` statement with a `let` head (`CopyEnv`,
//! CreatePerIterationEnvironment, `#sec-createperiterationenvironment`, one step per slot
//! copied). The `let` and `const` bindings of a script live in the global environment's
//! [[DeclarativeRecord]] (`env/global.rs`), which `InitGlobal` initializes.

use crate::env::{EnvironmentRecord, NameTable, RecordNames, SlotNames};
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::interp::frame::SlotValue;
use crate::object::property::Key;
use alloc::string::String;

impl<H: Host> Agent<H> {
    /// String constant `index` of the running code as Rust text, for a message.
    fn lexical_text(&mut self, index: u32) -> String {
        let position = usize::try_from(index).unwrap_or(usize::MAX);
        self.interp
            .frames
            .last()
            .and_then(|frame| frame.compiled.code.strings.get(position))
            .map(|units| String::from_utf16_lossy(units))
            .unwrap_or_default()
    }

    /// The property key of String constant `index` of the running code.
    fn lexical_key(&mut self, index: u32) -> Completion<Key> {
        let position = usize::try_from(index).unwrap_or(usize::MAX);
        let found = self.top_frame()?.compiled.keys.get(position).copied();
        match found {
            Some(key) => Ok(key),
            None => Err(self.fault()),
        }
    }

    /// What local slot `slot` of the running frame holds.
    fn local_state(&mut self, slot: u32) -> Completion<SlotValue> {
        let position = usize::try_from(slot).unwrap_or(usize::MAX);
        let state = self.top_frame()?.locals.get(position).copied();
        match state {
            Some(state) => Ok(state),
            None => Err(self.fault()),
        }
    }

    /// Stores `state` into local slot `slot` of the running frame.
    fn store_local(&mut self, slot: u32, state: SlotValue) -> Completion<()> {
        let position = usize::try_from(slot).unwrap_or(usize::MAX);
        let stored = self
            .top_frame()?
            .locals
            .get_mut(position)
            .map(|binding| *binding = state);
        match stored {
            Some(()) => Ok(()),
            None => Err(self.fault()),
        }
    }

    /// `Uninitialize(n)`.
    #[inline(never)]
    pub(crate) fn uninitialize(&mut self, slot: u32) -> Completion<()> {
        self.store_local(slot, SlotValue::Uninitialized)
    }

    /// `PushLexicalEnv(t)`: a declarative record of one uninitialized slot per name of
    /// `scopes[t]` of the running code, which names its slots.
    #[inline(never)]
    pub(crate) fn push_lexical_env(&mut self, table: u32) -> Completion<()> {
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
        let env = self.new_lexical_environment(outer, slots, names)?;
        let frame = self.top_frame()?;
        frame.env = Some(env);
        frame.pushed_envs = frame.pushed_envs.saturating_add(1);
        Ok(())
    }

    /// `CopyEnv`: CreatePerIterationEnvironment (`#sec-createperiterationenvironment`, steps
    /// 1.b to 1.e) of the LexicalEnvironment, which `PushLexicalEnv` made: a record with the
    /// same names, the same outer and a copy of the slots, one step per slot.
    #[inline(never)]
    pub(crate) fn copy_env(&mut self) -> Completion<()> {
        let Some(current) = self.top_frame()?.env else {
            return Err(self.fault());
        };
        let (outer, names, slots) = match self.environment(current)? {
            EnvironmentRecord {
                outer,
                names: Some(RecordNames::Slots(names)),
                slots,
                binding_object: None,
                ..
            } => (
                *outer,
                SlotNames {
                    script: names.script,
                    compiled: alloc::rc::Rc::clone(&names.compiled),
                    table: names.table,
                },
                slots.clone(),
            ),
            _ => return Err(self.fault()),
        };
        let count = u32::try_from(slots.len()).unwrap_or(u32::MAX);
        self.charge_steps(u64::from(count))?;
        let env = self.new_lexical_environment(outer, count, names)?;
        match self.heap.environments.get_mut(env) {
            Ok(record) => {
                for (copy, slot) in record.slots.iter_mut().zip(&slots) {
                    *copy = *slot;
                }
            }
            Err(_) => return Err(self.fault()),
        }
        self.top_frame()?.env = Some(env);
        Ok(())
    }

    /// `GetLocalChecked { slot, name }`.
    #[inline(never)]
    pub(crate) fn get_local_checked(&mut self, slot: u32, name: u32) -> Completion<()> {
        match self.local_state(slot)? {
            SlotValue::Value(value) => self.push_value(value),
            SlotValue::Uninitialized => {
                let text = self.lexical_text(name);
                Err(self.not_initialized(&text))
            }
            SlotValue::PrivateName(_) => Err(self.fault()),
        }
    }

    /// `SetLocalChecked { slot, name }`.
    #[inline(never)]
    pub(crate) fn set_local_checked(&mut self, slot: u32, name: u32) -> Completion<()> {
        let value = self.peek_value(0)?;
        match self.local_state(slot)? {
            SlotValue::Value(_) => self.store_local(slot, SlotValue::Value(value)),
            SlotValue::Uninitialized => {
                let text = self.lexical_text(name);
                Err(self.not_initialized(&text))
            }
            SlotValue::PrivateName(_) => Err(self.fault()),
        }
    }

    /// The `ReferenceError` of slot `slot` of the record `hops` outward, which is
    /// uninitialized.
    fn env_not_initialized(&mut self, hops: u32, slot: u32) -> Completion<()> {
        let env = self.top_frame()?.env;
        let handle = self.environment_at(env, hops)?;
        let text = self.slot_name(handle, slot);
        Err(self.not_initialized(&text))
    }

    /// `GetEnvChecked { hops, slot }`.
    #[inline(never)]
    pub(crate) fn get_env_checked(&mut self, hops: u32, slot: u32) -> Completion<()> {
        let env = self.top_frame()?.env;
        match self.slot_state(env, hops, slot)? {
            SlotValue::Value(value) => self.push_value(value),
            SlotValue::Uninitialized => self.env_not_initialized(hops, slot),
            SlotValue::PrivateName(_) => Err(self.fault()),
        }
    }

    /// `SetEnvChecked { hops, slot }`.
    #[inline(never)]
    pub(crate) fn set_env_checked(&mut self, hops: u32, slot: u32) -> Completion<()> {
        let value = self.peek_value(0)?;
        let env = self.top_frame()?.env;
        match self.slot_state(env, hops, slot)? {
            SlotValue::Value(_) => self.set_slot(env, hops, slot, value),
            SlotValue::Uninitialized => self.env_not_initialized(hops, slot),
            SlotValue::PrivateName(_) => Err(self.fault()),
        }
    }

    /// `AssignConstLocal { slot, name }`.
    #[inline(never)]
    pub(crate) fn assign_const_local(&mut self, slot: u32, name: u32) -> Completion<()> {
        let state = self.local_state(slot)?;
        let text = self.lexical_text(name);
        match state {
            SlotValue::Value(_) => self.assign_const(&text),
            SlotValue::Uninitialized => Err(self.not_initialized(&text)),
            SlotValue::PrivateName(_) => Err(self.fault()),
        }
    }

    /// `AssignConstEnv { hops, slot }`.
    #[inline(never)]
    pub(crate) fn assign_const_env(&mut self, hops: u32, slot: u32) -> Completion<()> {
        let env = self.top_frame()?.env;
        let state = self.slot_state(env, hops, slot)?;
        let handle = self.environment_at(env, hops)?;
        let text = self.slot_name(handle, slot);
        match state {
            SlotValue::Value(_) => self.assign_const(&text),
            SlotValue::Uninitialized => Err(self.not_initialized(&text)),
            SlotValue::PrivateName(_) => Err(self.fault()),
        }
    }

    /// `InitGlobal(k)`.
    #[inline(never)]
    pub(crate) fn init_global(&mut self, index: u32) -> Completion<()> {
        let value = self.peek_value(0)?;
        let key = self.lexical_key(index)?;
        let realm = self.top_frame()?.realm;
        self.initialize_global_lexical(realm, key, value)
    }
}
