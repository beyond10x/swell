//! The iteration methods of `Array.prototype` (ECMA-262 23.1.3,
//! `#sec-properties-of-the-array-prototype-object`), the group of
//! `story:array-iteration-methods`. It has no builtin yet: its id type has no variant, its
//! lists are empty and its run function has no arm (`story:builtin-group-modules`).

use crate::builtins::{Method, SymbolData};
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::value::{ObjectRef, Value};

/// The builtins of this group, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IterationBuiltin {}

/// The methods this group defines on `%Array%`, in the order of their clauses.
pub(crate) const CONSTRUCTOR_METHODS: &[Method<IterationBuiltin>] = &[];

/// The methods this group defines on `%Array.prototype%`, in the order of their clauses.
pub(crate) const PROTOTYPE_METHODS: &[Method<IterationBuiltin>] = &[];

/// The data properties keyed by a well-known symbol this group defines on
/// `%Array.prototype%`, after its methods.
pub(crate) fn prototype_symbol_data<H: Host>() -> [SymbolData<H>; 0] {
    []
}

impl<H: Host> Agent<H> {
    /// The steps of the builtins of this group; not inlined, so that the frame of
    /// `run_array_builtin` does not grow with its arms.
    #[inline(never)]
    pub(crate) fn run_array_iteration_builtin(
        &mut self,
        id: IterationBuiltin,
        _function: ObjectRef,
        _this: Value,
        _args: &[Value],
        _new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        match id {}
    }
}
