//! A frame of the engine's frame stack (`docs/design/engine.md`, decision 3: instruction index,
//! operand stack, local slots, environment, handler table; ECMA-262 9.4,
//! `#sec-execution-contexts`). Filled by `story:interpreter-and-script-entry-points`.
//!
//! A frame is an ECMAScript code execution context: of a script or of eval code (its Function is
//! null) or of a call of an ECMAScript function. It holds the code it runs (the script's
//! [`Compiled`] and, for a function, the index of its template), the instruction index, the
//! operand stack, the local slots, the LexicalEnvironment (`None` for the global environment of
//! the frame's realm) with the number of environments it made itself (`PushEnv`,
//! `PushNamedEnv`, `PushLexicalEnv`, `PushWith`), the VariableEnvironment, the `this`
//! value, the function, the realm, the script, and for a script the completion value register.
//!
//! What a frame takes beyond its fixed part (its locals and its operand stack as it grows) is
//! charged to the memory budget and credited when the frame is popped; so are the key list a
//! `for`-`in` enumerator on its operand stack holds and the box of a `for`-`of` Iterator
//! Record there, each credited earlier when its entry leaves the stack.

use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::vec::Vec;

use swell_compiler::bytecode::{Handler, Instruction};

use crate::env::NameReference;
use crate::heap::arena::Handle;
use crate::heap::collect::{Item, Mark};
use crate::object::property::Key;
use crate::ops::iteration::IteratorRecord;
use crate::script::Compiled;
use crate::value::{ObjectRef, Value};

/// An entry of the operand stack: a language value, or one of the five engine entries of the
/// bytecode (`swell_compiler::bytecode`, "Values on the operand stack").
pub(crate) enum Entry {
    Value(Value),
    /// The Reference Record of a global name (`ResolveGlobal`), or of a name looked up at run
    /// time (`ResolveName`) that is unresolvable or that the global environment binds: whether
    /// it is resolvable, its [[Base]] then being the global environment.
    Reference(bool),
    /// The Reference Record of a name looked up at run time (`ResolveName`) whose [[Base]] is
    /// this declarative record.
    DeclarativeReference(Handle),
    /// The Reference Record of a name looked up at run time whose [[Base]] is the Object
    /// Environment Record of a `with` statement with this binding object.
    ObjectReference(ObjectRef),
    /// The enumerator of a for-in statement (`ForInStart`).
    Enumerator(Box<Enumerator>),
    /// The Iterator Record of a for-of statement (`GetIterator`), boxed so the entry stays as
    /// large as it was (`interp/for_of.rs`).
    Iterator(Box<IteratorRecord>),
    /// The tag of a completion record (`Tag`).
    Tag(u32),
}

impl Entry {
    /// The entry of the Reference Record `reference`.
    pub(crate) fn of_name(reference: NameReference) -> Entry {
        match reference {
            NameReference::Unresolvable => Entry::Reference(false),
            NameReference::Global => Entry::Reference(true),
            NameReference::Declarative(env) => Entry::DeclarativeReference(env),
            NameReference::Object(object) => Entry::ObjectReference(object),
        }
    }

    /// The Reference Record of a name the entry holds, if it holds one.
    pub(crate) fn name_reference(&self) -> Option<NameReference> {
        match *self {
            Entry::Reference(false) => Some(NameReference::Unresolvable),
            Entry::Reference(true) => Some(NameReference::Global),
            Entry::DeclarativeReference(env) => Some(NameReference::Declarative(env)),
            Entry::ObjectReference(object) => Some(NameReference::Object(object)),
            Entry::Value(_) | Entry::Enumerator(_) | Entry::Iterator(_) | Entry::Tag(_) => None,
        }
    }
}

/// What a local slot or a slot of a declarative record holds: a value, or the uninitialized
/// state of a `let` or `const` binding before its declaration ran (`docs/design/engine.md`,
/// decision 3, "Lexical bindings", item 2). A type of the engine that no host interface and no
/// builtin receives; the same size as a [`Value`], so a slot costs what it cost before.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum SlotValue {
    Value(Value),
    Uninitialized,
}

impl SlotValue {
    /// The value, when the slot holds one.
    pub(crate) fn value(self) -> Option<Value> {
        match self {
            SlotValue::Value(value) => Some(value),
            SlotValue::Uninitialized => None,
        }
    }
}

/// The state of a for-in statement's enumeration: the object and its keys as
/// EnumerateObjectProperties (`#sec-enumerate-object-properties`) gave them, and the next one.
/// `object` is `None` for the empty enumerator of `undefined` and `null`. `bytes` is what the
/// list of keys was charged (0 for the empty enumerator): it is part of the frame's `charged`
/// while the enumerator is on the operand stack, and credited when it leaves it
/// (`Agent::truncate_stack`) or with the frame.
pub(crate) struct Enumerator {
    pub(crate) object: Option<ObjectRef>,
    pub(crate) keys: Vec<Key>,
    pub(crate) next: usize,
    pub(crate) bytes: u64,
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
    /// The local slots of the code; for a call of a function whose parameter list is not
    /// simple the arguments follow them, kept for the code that binds the parameters
    /// (`interp/parameters.rs`).
    pub(crate) locals: Vec<SlotValue>,
    /// The LexicalEnvironment: a record of the `environments` arena, or `None` for the global
    /// environment of `realm`.
    pub(crate) env: Option<Handle>,
    /// The VariableEnvironment, where a direct `eval` of non-strict code adds its `var` and
    /// function bindings (`#sec-performeval`, `#sec-evaldeclarationinstantiation`): `None` for
    /// the global environment of `realm` (a script), the record the call made for a function
    /// (one whose code contains a direct `eval` always makes one, `swell-compiler`'s
    /// `resolve.rs`; while the parameters of non-strict code with an initializer that calls
    /// `eval` directly are bound, the empty record outside it, and from `PushVarEnv` on the
    /// record of the body's `var` names), and the caller's, or the record of its own for strict
    /// eval code, for eval code.
    pub(crate) var_env: Option<Handle>,
    /// How many of the environments on the way out from `env` this frame made (`PushEnv`,
    /// `PushNamedEnv`, `PushLexicalEnv`, `PushVarEnv` and `PushWith`).
    pub(crate) pushed_envs: u32,
    /// The `this` binding (ResolveThisBinding, `#sec-resolvethisbinding`).
    pub(crate) this: Value,
    /// The function GetThisEnvironment answers (`#sec-getthisenvironment`, [[FunctionObject]]):
    /// the callee; for an arrow function the function its box kept, that of the code that made
    /// it (`docs/design/engine.md`, decision 3, "`this` and `arguments` in an arrow function",
    /// item 4); for direct eval code the caller frame's (item 5); `None` for a script, an
    /// indirect eval's code and an arrow made outside function code.
    pub(crate) function: Option<ObjectRef>,
    /// [[NewTarget]] (`#sec-function-environment-records`, GetNewTarget `#sec-getnewtarget`):
    /// the NewTarget of a [[Construct]], set before the code runs and never changed; for an
    /// arrow function the value its box kept, for direct eval code the caller frame's; `None`
    /// (`undefined`) for every other frame.
    pub(crate) new_target: Option<ObjectRef>,
    /// The Realm component.
    pub(crate) realm: Handle,
    /// The ScriptOrModule component.
    pub(crate) script: Handle,
    /// Whether the code is strict mode code.
    pub(crate) strict: bool,
    /// The completion value register of a script.
    pub(crate) completion: Value,
    /// Which [[Construct]] the frame runs, if any ("Classes", item 8, `docs/design/engine.md`),
    /// which decides what its return answers ([`Constructed`]).
    pub(crate) constructed: Constructed,
    /// The bytes charged for the frame, the key lists of the enumerators and the Iterator
    /// Records on its operand stack, credited when it is popped.
    pub(crate) charged: u64,
}

/// Which [[Construct]] a frame runs (`#sec-ecmascript-function-objects-construct-argumentslist-newtarget`;
/// `docs/design/engine.md`, "Classes", item 8), as large as an `Option<ObjectRef>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Constructed {
    /// A call, eval code or a script: the return answers its value.
    None,
    /// The [[Construct]] of a base constructor, with the `this` object made for it: a return
    /// with no object answers that object (step 10).
    Base(ObjectRef),
    /// The [[Construct]] of a derived constructor, which made no object: its `this` is the
    /// binding of its code, which `KeepThis` copies into the frame's `this` before a return;
    /// the return answers by steps 10 to 12 (`Agent::derived_result`).
    Derived,
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
                Entry::Iterator(record) => {
                    mark.object(record.iterator);
                    mark.value(record.next_method);
                }
                Entry::DeclarativeReference(env) => mark.push(Item::Environment(*env)),
                Entry::ObjectReference(object) => mark.object(*object),
                Entry::Reference(_) | Entry::Tag(_) => {}
            }
        }
        for value in self.locals.iter().filter_map(|slot| slot.value()) {
            mark.value(value);
        }
        if let Some(env) = self.env {
            mark.push(Item::Environment(env));
        }
        if let Some(env) = self.var_env {
            mark.push(Item::Environment(env));
        }
        mark.value(self.this);
        if let Some(function) = self.function {
            mark.object(function);
        }
        if let Some(new_target) = self.new_target {
            mark.object(new_target);
        }
        mark.push(Item::Realm(self.realm));
        mark.push(Item::Script(self.script));
        mark.value(self.completion);
        if let Constructed::Base(object) = self.constructed {
            mark.object(object);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SlotValue;
    use crate::value::Value;

    /// Decision 3, "Lexical bindings", item 2: the uninitialized state takes no room of its own,
    /// so a local slot and a record's slot are charged as a `Value` was.
    #[test]
    fn a_slot_is_as_large_as_a_value() {
        assert_eq!(size_of::<SlotValue>(), size_of::<Value>());
        assert_eq!(size_of::<SlotValue>(), 16);
    }

    /// "Classes", item 8: the three states of a frame's `constructed` (no [[Construct]], the
    /// object of a base constructor, a derived constructor) take the room of an
    /// `Option<ObjectRef>`, so `frame_bytes()` does not grow.
    #[test]
    fn constructed_is_as_large_as_an_option_of_an_object() {
        use super::Constructed;
        use crate::value::ObjectRef;
        assert_eq!(size_of::<Constructed>(), size_of::<Option<ObjectRef>>());
    }
}
