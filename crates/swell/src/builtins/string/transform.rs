//! The transform and compare methods of `String` and `String.prototype` (ECMA-262 22.1.2 and
//! 22.1.3, `#sec-properties-of-the-string-constructor`,
//! `#sec-properties-of-the-string-prototype-object`), the group of
//! `story:string-transform-and-compare-methods`. It has no builtin yet: its id type has no
//! variant, its lists are empty and its run function has no arm
//! (`story:builtin-group-modules`).

use crate::builtins::{Method, SymbolData};
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::value::{ObjectRef, Value};

/// The builtins of this group, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TransformBuiltin {}

/// The methods this group defines on `%String%`, in the order of their clauses.
pub(crate) const CONSTRUCTOR_METHODS: &[Method<TransformBuiltin>] = &[];

/// The methods this group defines on `%String.prototype%`, in the order of their clauses.
pub(crate) const PROTOTYPE_METHODS: &[Method<TransformBuiltin>] = &[];

/// The data properties keyed by a well-known symbol this group defines on
/// `%String.prototype%`, after its methods.
pub(crate) fn prototype_symbol_data<H: Host>() -> [SymbolData<H>; 0] {
    []
}

impl<H: Host> Agent<H> {
    /// The steps of the builtins of this group; not inlined, so that the frame of
    /// `run_string_builtin` does not grow with its arms.
    #[inline(never)]
    pub(crate) fn run_string_transform_builtin(
        &mut self,
        id: TransformBuiltin,
        _function: ObjectRef,
        _this: Value,
        _args: &[Value],
        _new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        match id {}
    }
}
