//! The integrity and prototype methods of `Object` and `Object.prototype` (ECMA-262 20.1.2 and
//! 20.1.3, `#sec-properties-of-the-object-constructor`,
//! `#sec-properties-of-the-object-prototype-object`), the group of
//! `story:object-integrity-and-prototype-methods`. It has no builtin yet: its id type has no
//! variant, its lists are empty and its run function has no arm
//! (`story:builtin-group-modules`).

use crate::builtins::{Method, SymbolData};
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::value::{ObjectRef, Value};

/// The builtins of this group, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IntegrityBuiltin {}

/// The methods this group defines on `%Object%`, in the order of their clauses.
pub(crate) const CONSTRUCTOR_METHODS: &[Method<IntegrityBuiltin>] = &[];

/// The methods this group defines on `%Object.prototype%`, in the order of their clauses.
pub(crate) const PROTOTYPE_METHODS: &[Method<IntegrityBuiltin>] = &[];

/// The data properties keyed by a well-known symbol this group defines on
/// `%Object.prototype%`, after its methods.
pub(crate) fn prototype_symbol_data<H: Host>() -> [SymbolData<H>; 0] {
    []
}

impl<H: Host> Agent<H> {
    /// The steps of the builtins of this group; not inlined, so that the frame of
    /// `run_object_builtin` does not grow with its arms.
    #[inline(never)]
    pub(crate) fn run_object_integrity_builtin(
        &mut self,
        id: IntegrityBuiltin,
        _function: ObjectRef,
        _this: Value,
        _args: &[Value],
        _new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        match id {}
    }
}
