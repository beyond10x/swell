//! A frame of the engine's frame stack (`docs/design/engine.md`, decision 3: instruction index,
//! operand stack, local slots, environment, handler table; ECMA-262 9.4,
//! `#sec-execution-contexts`). Filled by `story:interpreter-and-script-entry-points`.
//!
//! A frame is an ECMAScript code execution context: of a script (its Function is null) or of a
//! call of an ECMAScript function. It holds the code it runs (the script's [`Compiled`] and, for a
//! function, the index of its template), the instruction index, the operand stack, the local
//! slots, the LexicalEnvironment (`None` for the global environment of the frame's realm) with
//! the number of environments its own `PushEnv`s made, the `this` value, the function, the
//! realm, the script, and for a script the completion value register.
//!
//! What a frame takes beyond its fixed part (its locals and its operand stack as it grows) is
//! charged to the memory budget and credited when the frame is popped.

use std::rc::Rc;

use swell_compiler::bytecode::{Handler, Instruction};

use crate::heap::arena::Handle;
use crate::heap::collect::{Item, Mark};
use crate::object::property::Key;
use crate::script::Compiled;
use crate::value::{ObjectRef, Value};

/// An entry of the operand stack: a language value, or one of the three engine entries of the
/// bytecode (`swell_compiler::bytecode`, "Values on the operand stack").
pub(crate) enum Entry {
    Value(Value),
    /// The Reference Record of a global name (`ResolveGlobal`): whether it is resolvable, its
    /// [[Base]] then being the global environment.
    Reference(bool),
    /// The enumerator of a for-in statement (`ForInStart`).
    Enumerator(Box<Enumerator>),
    /// The tag of a completion record (`Tag`).
    Tag(u32),
}

/// The state of a for-in statement's enumeration: the object and its keys as
/// EnumerateObjectProperties (`#sec-enumerate-object-properties`) gave them, and the next one.
/// `object` is `None` for the empty enumerator of `undefined` and `null`.
pub(crate) struct Enumerator {
    pub(crate) object: Option<ObjectRef>,
    pub(crate) keys: Vec<Key>,
    pub(crate) next: usize,
}

/// One ECMAScript code execution context.
pub(crate) struct Frame {
    /// The code of the script the frame runs code of.
    pub(crate) compiled: Rc<Compiled>,
    /// The template of the function the frame runs; `None` for the script's own code.
    pub(crate) template: Option<u32>,
    /// The index of the next instruction.
    pub(crate) pc: usize,
    pub(crate) stack: Vec<Entry>,
    pub(crate) locals: Vec<Value>,
    /// The LexicalEnvironment: a record of the `environments` arena, or `None` for the global
    /// environment of `realm`.
    pub(crate) env: Option<Handle>,
    /// How many of the environments on the way out from `env` this frame's `PushEnv`s made.
    pub(crate) pushed_envs: u32,
    /// The `this` binding (ResolveThisBinding, `#sec-resolvethisbinding`).
    pub(crate) this: Value,
    /// The Function component: the callee, `None` for a script.
    pub(crate) function: Option<ObjectRef>,
    /// The Realm component.
    pub(crate) realm: Handle,
    /// The ScriptOrModule component.
    pub(crate) script: Handle,
    /// Whether the code is strict mode code.
    pub(crate) strict: bool,
    /// The completion value register of a script.
    pub(crate) completion: Value,
    /// For a [[Construct]], the `this` object the call answers when its code returns no object.
    pub(crate) constructed: Option<ObjectRef>,
    /// The bytes charged for the frame, credited when it is popped.
    pub(crate) charged: u64,
}

/// The bytes a frame charges besides its locals and its operand stack.
pub(crate) fn frame_bytes() -> u64 {
    u64::try_from(size_of::<Frame>()).unwrap_or(u64::MAX)
}

/// The bytes one operand stack entry takes.
pub(crate) fn entry_bytes() -> u64 {
    u64::try_from(size_of::<Entry>()).unwrap_or(u64::MAX)
}

impl Frame {
    /// The instructions the frame runs.
    pub(crate) fn instructions(&self) -> &[Instruction] {
        match self.template {
            None => &self.compiled.code.instructions,
            Some(index) => self
                .compiled
                .code
                .functions
                .get(usize::try_from(index).unwrap_or(usize::MAX))
                .map_or(&[], |template| template.instructions.as_slice()),
        }
    }

    /// The handler table of the code the frame runs.
    pub(crate) fn handlers(&self) -> &[Handler] {
        match self.template {
            None => &self.compiled.code.handlers,
            Some(index) => self
                .compiled
                .code
                .functions
                .get(usize::try_from(index).unwrap_or(usize::MAX))
                .map_or(&[], |template| template.handlers.as_slice()),
        }
    }

    /// What the frame holds, for the root site of `interp/mod.rs`: the operand stack, the
    /// locals, the environment, `this`, the function, the realm, the script (which holds the
    /// constants of the code), the completion value and the constructed object.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        for entry in &self.stack {
            match entry {
                Entry::Value(value) => mark.value(*value),
                Entry::Enumerator(enumerator) => {
                    if let Some(object) = enumerator.object {
                        mark.object(object);
                    }
                    for key in &enumerator.keys {
                        key.trace(mark);
                    }
                }
                Entry::Reference(_) | Entry::Tag(_) => {}
            }
        }
        for &value in &self.locals {
            mark.value(value);
        }
        if let Some(env) = self.env {
            mark.push(Item::Environment(env));
        }
        mark.value(self.this);
        if let Some(function) = self.function {
            mark.object(function);
        }
        mark.push(Item::Realm(self.realm));
        mark.push(Item::Script(self.script));
        mark.value(self.completion);
        if let Some(object) = self.constructed {
            mark.object(object);
        }
    }
}
