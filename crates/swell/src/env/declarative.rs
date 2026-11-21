//! Declarative Environment Records (ECMA-262 9.1.1.1, `#sec-declarative-environment-records`).
//! Filled by `story:interpreter-and-script-entry-points`.
//!
//! A record is the slots of [`crate::env::EnvironmentRecord`]: the compiler numbered the
//! bindings, so HasBinding and the binding's name are answered at compile time, and
//! GetBindingValue and SetMutableBinding read and write a slot (`env/mod.rs`). Every binding of
//! slice 1 is created initialized (a `var`, a parameter, a catch parameter or a function's own
//! name), so none is ever uninitialized; `let`, `const` and `class` bring uninitialized ones
//! with slice 6. This file holds NewDeclarativeEnvironment and the one rule the slots do not
//! carry, the assignment to an immutable binding.

use crate::heap::arena::Handle;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};

impl<H: Host> Agent<H> {
    /// NewDeclarativeEnvironment (`#sec-newdeclarativeenvironment`) with `slots` bindings, each
    /// created and initialized to `undefined` (CreateMutableBinding and InitializeBinding of a
    /// catch parameter, `#sec-runtime-semantics-catchclauseevaluation`, or CreateImmutableBinding
    /// of a named function expression's name,
    /// `#sec-runtime-semantics-instantiateordinaryfunctionexpression`).
    pub(crate) fn new_declarative_environment(
        &mut self,
        outer: Option<Handle>,
        slots: u32,
    ) -> Completion<Handle> {
        self.new_environment(outer, slots)
    }

    /// SetMutableBinding of an immutable binding
    /// (`#sec-declarative-environment-records-setmutablebinding-n-v-s`, step 5: "Else if the
    /// binding for N in envRec is a mutable binding, ... Else, Assert: This is an attempt to
    /// change the value of an immutable binding. If S is true, throw a TypeError exception."):
    /// in strict code a `TypeError`, otherwise nothing.
    pub(crate) fn assign_immutable(&mut self, strict: bool, name: &str) -> Completion<()> {
        if strict {
            let message = format!("Assignment to the constant binding {name}");
            return Err(self.throw_error(ErrorKind::TypeError, &message));
        }
        Ok(())
    }
}
