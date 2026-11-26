//! Declarative Environment Records (ECMA-262 9.1.1.1, `#sec-declarative-environment-records`).
//! Filled by `story:interpreter-and-script-entry-points`.
//!
//! A record is the slots of [`crate::env::EnvironmentRecord`]: the compiler numbered the
//! bindings, so HasBinding and the binding's name are answered at compile time (but for a
//! lookup by name, which reads the names a record's slots have, `env/mod.rs`), and
//! GetBindingValue and SetMutableBinding read and write a slot (`env/mod.rs`). A `var`, a
//! parameter, a catch parameter and a function's own name are created initialized; a `let` or
//! `const` binding is uninitialized until its declaration runs (slice 6), in a record
//! [`Agent::new_lexical_environment`] makes. This file holds NewDeclarativeEnvironment and the
//! rules the slots do not carry: the assignment to an immutable binding, to a `const` one, and
//! the access to an uninitialized one.

use crate::env::{RecordNames, SlotNames};
use crate::heap::arena::Handle;
use crate::host::agent::Agent;
use crate::host::outcome::{Abrupt, Completion};
use crate::host::{ErrorKind, Host};
use crate::interp::frame::SlotValue;
use alloc::format;

impl<H: Host> Agent<H> {
    /// NewDeclarativeEnvironment (`#sec-newdeclarativeenvironment`) with `slots` bindings, each
    /// created and initialized to `undefined` (CreateMutableBinding and InitializeBinding of a
    /// catch parameter, `#sec-runtime-semantics-catchclauseevaluation`, or CreateImmutableBinding
    /// of a named function expression's name,
    /// `#sec-runtime-semantics-instantiateordinaryfunctionexpression`).
    /// `names` names the slots when a lookup by name may reach them (`env/mod.rs`).
    pub(crate) fn new_declarative_environment(
        &mut self,
        outer: Option<Handle>,
        slots: u32,
        names: Option<SlotNames>,
    ) -> Completion<Handle> {
        self.new_environment(outer, slots, names)
    }

    /// NewDeclarativeEnvironment with `slots` `let` and `const` bindings named by `names`, each
    /// created uninitialized (CreateMutableBinding(N, false) and CreateImmutableBinding(N, true)
    /// of BlockDeclarationInstantiation, `#sec-blockdeclarationinstantiation`, and of the other
    /// scopes of decision 3, "Lexical bindings").
    pub(crate) fn new_lexical_environment(
        &mut self,
        outer: Option<Handle>,
        slots: u32,
        names: SlotNames,
    ) -> Completion<Handle> {
        self.new_environment_of(
            outer,
            slots,
            Some(RecordNames::Slots(names)),
            SlotValue::Uninitialized,
        )
    }

    /// SetMutableBinding of an immutable binding
    /// (`#sec-declarative-environment-records-setmutablebinding-n-v-s`, step 5: "Else if the
    /// binding for N in envRec is a mutable binding, ... Else, Assert: This is an attempt to
    /// change the value of an immutable binding. If S is true, throw a TypeError exception."):
    /// in strict code a `TypeError`, otherwise nothing.
    pub(crate) fn assign_immutable(&mut self, strict: bool, name: &str) -> Completion<()> {
        if strict {
            return self.assign_const(name);
        }
        Ok(())
    }

    /// SetMutableBinding of an initialized `const` binding, which is strict (step 5 with S
    /// true, CreateImmutableBinding(N, true)): a `TypeError` in any code.
    pub(crate) fn assign_const(&mut self, name: &str) -> Completion<()> {
        let message = format!("Assignment to the constant binding {name}");
        Err(self.throw_error(ErrorKind::TypeError, &message))
    }

    /// The `ReferenceError` of an access to the uninitialized binding `name`
    /// (`#sec-declarative-environment-records-getbindingvalue-n-s`, step 2;
    /// `#sec-declarative-environment-records-setmutablebinding-n-v-s`, step 3).
    pub(crate) fn not_initialized(&mut self, name: &str) -> Abrupt {
        let message = format!("{name} is not initialized");
        self.throw_error(ErrorKind::ReferenceError, &message)
    }
}
